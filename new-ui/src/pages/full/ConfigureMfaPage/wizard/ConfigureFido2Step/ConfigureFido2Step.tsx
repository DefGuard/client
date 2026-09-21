import './style.scss';
import { Enter } from '@fluentui/keyboard-keys';
import { useMutation } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { hostname } from '@tauri-apps/plugin-os';
import { Fragment, useCallback, useEffect, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Input } from '../../../../../shared/components/Input/Input';
import { Fido2TouchPrompt } from '../../../../../shared/components/LocationCard/components/Fido2TouchPrompt/Fido2TouchPrompt';
import { api } from '../../../../../shared/rust-api/api';
import {
  fido2CollectsPinInApp,
  fido2ShowsTouchPrompt,
} from '../../../../../shared/rust-api/fido2';
import { MfaMethod, TauriEvent } from '../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { useMfaConfigErrorHandler } from '../../hooks/useMfaConfigErrorHandler';

const DEFAULT_KEY_NAME = 'Security key';

/** A second key registered from the same machine would otherwise propose the name the first one
 *  took, leaving two entries the user cannot tell apart in Defguard. */
const defaultKeyName = (host: string | null | undefined, hasKey: boolean): string => {
  const base = isPresent(host) ? `${host} key` : DEFAULT_KEY_NAME;
  return hasKey ? `${base} (${new Date().toISOString().slice(0, 10)})` : base;
};

interface Props {
  onCancel: () => void;
  /** The session outlived its deadline, so the whole flow has to start over. */
  onSessionExpired: () => void;
}

/** Registers a security key, offered however many the account already holds. */
export const ConfigureFido2Step = ({ onCancel, onSessionExpired }: Props) => {
  const sessionId = useConfigureMfaStore((s) => s.sessionId);
  // The snapshot is frozen at the start of the session, so this still reads false on the first
  // key even after it is registered.
  const hasKeyAlready = useConfigureMfaStore((s) =>
    s.configuredMethods.includes(MfaMethod.Fido2),
  );

  const [name, setName] = useState<string | null>(null);
  const [pin, setPin] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isAwaitingTouch, setIsAwaitingTouch] = useState(false);
  const collectsPin = fido2CollectsPinInApp();

  // The machine's own name is what the user will recognize the key by in Defguard.
  useEffect(() => {
    void hostname().then((host) => {
      setName((current) => current ?? defaultKeyName(host, hasKeyAlready));
    });
  }, [hasKeyAlready]);

  const handleApiError = useMfaConfigErrorHandler({
    context: 'Security key registration failed',
    setError,
    onSessionExpired,
    fallback: 'Registration failed',
    hasCodeInput: false,
  });

  const { mutate: register, isPending: isRegistering } = useMutation({
    mutationFn: async () => {
      if (!isPresent(sessionId)) {
        throw new Error('No MFA configuration session');
      }
      // Listen before starting, or a registration that fails fast is never heard from.
      const unlisten = await listen(TauriEvent.MfaConfigFido2Touch, () => {
        // A platform running the ceremony shows its own prompt, ours would render behind it.
        setIsAwaitingTouch(fido2ShowsTouchPrompt());
      });
      try {
        return await api.mfaConfigSetupFido2(
          sessionId,
          name ?? '',
          collectsPin ? pin : null,
        );
      } finally {
        unlisten();
        setIsAwaitingTouch(false);
      }
    },
    onError: handleApiError,
    onSuccess: (result) => {
      const store = useConfigureMfaStore.getState();
      store.factorConfigured(MfaMethod.Fido2, result.recovery_codes);
      store.next();
    },
  });

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

  const handleSubmit = useCallback(() => {
    if (isRegistering || isCancelling) return;
    if (!isPresent(name) || name.trim().length === 0) {
      setError('Name your security key');
      return;
    }
    if (collectsPin && (!isPresent(pin) || pin.length === 0)) {
      setError('Enter PIN');
      return;
    }
    setError(null);
    register();
  }, [collectsPin, isCancelling, isRegistering, name, pin, register]);

  return (
    <div
      id="configure-fido2-step"
      className="step-content"
      onKeyDown={(e) => {
        if (e.key === Enter) handleSubmit();
      }}
    >
      <header>
        <h1>Register a security key</h1>
        <p>
          {collectsPin
            ? `Insert your security key, name it so you can recognize it later, and enter its PIN.`
            : `Insert your security key and name it so you can recognize it later, then continue in the prompt your system shows.`}
        </p>
      </header>
      {isAwaitingTouch ? (
        <Fido2TouchPrompt />
      ) : (
        <Fragment>
          <Input
            label="Security key name"
            value={name}
            onChange={(value) => setName(isPresent(value) ? String(value) : null)}
            error={collectsPin ? undefined : error}
          />
          {collectsPin && (
            <Input
              type="password"
              label="PIN"
              value={pin}
              onChange={(value) => setPin(isPresent(value) ? String(value) : null)}
              error={error}
            />
          )}
        </Fragment>
      )}
      <Controls>
        <Button
          text="Cancel"
          variant={ButtonVariant.Secondary}
          loading={isCancelling}
          onClick={() => {
            cancel();
          }}
        />
        <div className="right">
          <Button
            text="Register"
            variant={ButtonVariant.Primary}
            loading={isRegistering}
            disabled={isCancelling}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
