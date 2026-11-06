import { useMutation } from '@tanstack/react-query';
import clsx from 'clsx';
import { fork } from 'radashi';
import { useCallback, useMemo, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { FactorSelector } from '../../../../../shared/components/FactorSelector/FactorSelector';
import { FieldError } from '../../../../../shared/components/FieldError/FieldError';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { Icon, IconKind } from '../../../../../shared/components/Icon';
import { SelectionItemVariant } from '../../../../../shared/components/SelectionItem/types';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { TooltipContent } from '../../../../../shared/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../shared/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../shared/providers/tooltip/TooltipTrigger';
import {
  MfaMethod,
  type MfaMethodValue,
  type MfaStep,
} from '../../../../../shared/rust-api/types';
import { ThemeVariable } from '../../../../../shared/types';
import { isMfaEditConfigureFactorsSource } from '../../../../../shared/utils/configureFactorsSource';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  CLIENT_CONFIGURABLE_METHODS,
  isClientConfigurableMethod,
  isDesktopDrivableMethod,
  isMfaMethodAvailable,
  isMfaMethodLocked,
  mfaStepsOf as locationMfaSteps,
  mfaToText,
  setupMethodsOf,
} from '../../../../../shared/utils/mfa';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import { isMfaFactorOfferable } from '../../utils';
import '../style.scss';
import './style.scss';

/** What one row has to say, resolved by the step so the markup only reads it. */
type MethodRowState = {
  checked: boolean;
  /** Nothing left for the user to decide here, the badge and the hint say why. */
  disabled: boolean;
  badge?: string;
  /** Why the row is disabled, shown on hover when the badge alone is too terse. */
  hint: string | null;
  /** The instance withholds the factor. */
  locked: boolean;
};

interface Props {
  onCancel: () => Promise<void>;
}

export const ConfigureSelectMethodsStep = ({ onCancel }: Props) => {
  const configuredMethods = useConfigureMfaStore((s) => s.configuredMethods);
  const emailFallback = useConfigureMfaStore((s) => s.emailFallback);
  const location = useConfigureMfaStore((s) => s.location);
  const initialSelection = useConfigureMfaStore((s) => s.initialSelection);
  const instance = useConfigureMfaStore((s) => s.instance);
  const source = useConfigureMfaStore((s) => s.source);

  const setupMethods = useMemo(() => setupMethodsOf(instance), [instance]);

  // Listing order, matching what `toggle` keeps.
  const [selected, setSelected] = useState<MfaMethodValue[]>(() =>
    setupMethods.filter((method) => initialSelection.includes(method)),
  );
  const [error, setError] = useState<string | null>(null);

  const locationSteps = useMemo(
    () => (isPresent(location) ? locationMfaSteps(location) : []),
    [location],
  );
  // Editing a location's defaults already picked the factors, so it gets the Add page list.
  const isLocationAware =
    locationSteps.length > 0 && !isMfaEditConfigureFactorsSource(source);

  /** The fallback registers email as it mails the code, so email is disabled but not
   *  something the account already holds. */
  const accountConfigured = useMemo(() => {
    if (!emailFallback) return configuredMethods;
    return configuredMethods.filter((method) => method !== MfaMethod.Email);
  }, [emailFallback, configuredMethods]);

  /** A step the user is already past: it offers a factor the desktop can drive, the account
   *  holds and the instance runs. Mirrors usableMfaMethods, on the session's fresher list. */
  const isStepSatisfied = useCallback(
    (step: MfaStep): boolean =>
      step.methods.some(
        (entry) =>
          isDesktopDrivableMethod(entry.method) &&
          configuredMethods.includes(entry.method) &&
          isMfaMethodAvailable(entry.method, instance),
      ),
    [configuredMethods, instance],
  );

  const groups = useMemo(
    () => locationSteps.map((step) => step.methods.map((entry) => entry.method)),
    [locationSteps],
  );

  const [availableMethods, lockedMethods] = useMemo(
    () =>
      fork(CLIENT_CONFIGURABLE_METHODS, (method) =>
        isMfaFactorOfferable(method, configuredMethods, setupMethods, source),
      ),
    [configuredMethods, setupMethods, source],
  );

  const describeMethod = useCallback(
    (method: MfaMethodValue): MethodRowState => {
      const disabled = !isMfaFactorOfferable(
        method,
        configuredMethods,
        setupMethods,
        source,
      );
      const configured = accountConfigured.includes(method);
      const available = isMfaMethodAvailable(method, instance);
      const satisfied = configuredMethods.includes(method) && available;
      const locked = isMfaMethodLocked(method, instance);
      const methodName = mfaToText(method, instance ?? undefined);

      let hint: string | null = null;
      let badge: string | undefined;
      if (!available || locked) {
        hint = `${methodName} is not available on this Defguard instance.`;
        badge = 'Unavailable';
      } else if (disabled && !configured) {
        if (satisfied) {
          // Reached only via the email fallback, which registers the factor as it verifies.
          hint = `${methodName} is required to continue.`;
          badge = 'Required';
        } else if (isClientConfigurableMethod(method)) {
          hint = `This Defguard instance does not support configuring ${methodName} from the desktop client.`;
          badge = 'Not configurable';
        } else {
          hint = `${methodName} cannot be configured in the desktop client.`;
          badge = 'Not configurable';
        }
      }
      if (configured) {
        badge = 'Configured';
      }

      return {
        checked: !disabled && selected.includes(method),
        disabled,
        badge,
        hint,
        locked,
      };
    },
    [accountConfigured, configuredMethods, setupMethods, source, selected, instance],
  );

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: onCancel,
  });

  const toggle = useCallback(
    (method: MfaMethodValue) => {
      setError(null);
      // Kept in listing order, which is the order the wizard sets them up in.
      setSelected((current) => {
        if (current.includes(method)) {
          return current.filter((picked) => picked !== method);
        }
        return setupMethods.filter(
          (candidate) => candidate === method || current.includes(candidate),
        );
      });
    },
    [setupMethods],
  );

  const handleSubmit = useCallback(() => {
    if (isLocationAware) {
      const unanswered = locationSteps.findIndex(
        (step) =>
          !isStepSatisfied(step) &&
          !step.methods.some((entry) => selected.includes(entry.method)),
      );
      if (unanswered !== -1) {
        setError(
          locationSteps.length > 1
            ? `Select a method for step ${unanswered + 1}`
            : 'Select at least one method',
        );
        return;
      }
      // Every step reads as answered yet there is nothing to set up, so continuing would drop
      // the user on the closing screen having configured nothing. The fallback configures email
      // on its own, so there it may be all the location asks for.
      if (selected.length === 0 && !emailFallback) {
        setError('Select at least one method');
        return;
      }
    } else if (selected.length === 0 && !emailFallback) {
      // The fallback configures email on its own, so it satisfies the one-factor minimum.
      setError('Select at least one method');
      return;
    }
    useConfigureMfaStore.getState().selectMethods(selected);
  }, [selected, emailFallback, isLocationAware, locationSteps, isStepSatisfied]);

  /** Without `sectionExplained` the row drops its badge and hint, leaving the reason to its section. */
  const renderMethod = (method: MfaMethodValue, sectionExplained: boolean) => {
    const { checked, disabled, badge, hint, locked } = describeMethod(method);
    // The "Not configurable" section text blames version incompatibility, so a withheld row says it.
    const explained = sectionExplained || locked;
    let rowBadge: string | undefined;
    if (explained) rowBadge = badge;
    const row = (
      <FactorSelector
        key={method}
        method={method}
        instance={instance ?? undefined}
        variant={SelectionItemVariant.Checkbox}
        selected={checked}
        disabled={disabled}
        badge={rowBadge}
        onClick={() => {
          toggle(method);
        }}
      />
    );
    if (!explained || !isPresent(hint)) return row;
    return (
      <TooltipProvider key={method}>
        <TooltipTrigger>{row}</TooltipTrigger>
        <TooltipContent>
          <p>{hint}</p>
        </TooltipContent>
      </TooltipProvider>
    );
  };

  let title = 'Select the MFA method(s) to configure';
  let description = 'Pick the multi-factor authentication methods you want to set up.';
  if (isLocationAware && isPresent(location)) {
    title = 'Set up missing MFA methods';
    description = `Choose an MFA method required by your administrator that you haven't configured yet. You can set up just the method needed to access ${location.name} or configure additional methods for future use.`;
  }

  return (
    <FullPage
      id="configure-select-methods-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title={title} />
      <p className="description">
        <span>{description}</span>
      </p>
      <div className={clsx('steps', { sections: !isLocationAware })}>
        {isLocationAware &&
          groups.map((methods, index) => (
            // Index as key: Core owns the order and it never reorders here.
            <div className="step" key={index}>
              {groups.length > 1 && (
                <div className="label">
                  <p>{`Step ${index + 1}:`}</p>
                </div>
              )}
              <div className="methods">
                {methods.map((method) => renderMethod(method, true))}
              </div>
            </div>
          ))}
        {!isLocationAware && (
          <>
            {availableMethods.length > 0 && (
              <div className="step">
                <div className="label">
                  <p>Available methods</p>
                </div>
                <div className="methods">
                  {availableMethods.map((method) => renderMethod(method, false))}
                </div>
              </div>
            )}
            {lockedMethods.length > 0 && (
              <div className="step">
                <div className="label">
                  <p>Not configurable</p>
                  <TooltipProvider>
                    <TooltipTrigger>
                      <Icon
                        icon={IconKind.Question}
                        size={16}
                        staticColor={ThemeVariable.FgWhite100}
                      />
                    </TooltipTrigger>
                    <TooltipContent>
                      <p>{`Some MFA methods are already configured or unavailable due to version incompatibility.`}</p>
                    </TooltipContent>
                  </TooltipProvider>
                </div>
                <div className="methods">
                  {lockedMethods.map((method) => renderMethod(method, false))}
                </div>
              </div>
            )}
          </>
        )}
      </div>
      <FieldError error={error} />
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
            text="Continue"
            variant={ButtonVariant.Primary}
            // Cancel already discarded the session, nothing left to continue into.
            disabled={isCancelling}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
