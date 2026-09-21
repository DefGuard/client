import { useMutation } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Checkbox } from '../../../../../shared/components/Checkbox/Checkbox';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { FieldError } from '../../../../../shared/components/FieldError/FieldError';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { TooltipContent } from '../../../../../shared/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../shared/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../shared/providers/tooltip/TooltipTrigger';
import { MfaMethod, type MfaMethodValue } from '../../../../../shared/rust-api/types';
import { mfaToText } from '../../../../../shared/utils/mfa';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { isMfaFactorOfferable, MFA_CONFIGURABLE_FACTORS } from '../../types';
import '../style.scss';
import './style.scss';

interface Props {
  onCancel: () => void;
}

const EMAIL_REQUIRED_TOOLTIP =
  'Email is required: it is the factor this session is being verified with.';

/** Opens the flow, the picks here are what the wizard sets up afterwards. */
export const ConfigureSelectMethodsStep = ({ onCancel }: Props) => {
  const configuredMethods = useConfigureMfaStore((s) => s.configuredMethods);
  const emailFallback = useConfigureMfaStore((s) => s.emailFallback);

  const [selected, setSelected] = useState<MfaMethodValue[]>([]);
  const [error, setError] = useState<string | null>(null);

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

  const toggle = useCallback((method: MfaMethodValue) => {
    setError(null);
    // Listing order is the order the wizard sets them up in.
    setSelected((current) =>
      current.includes(method)
        ? current.filter((picked) => picked !== method)
        : MFA_CONFIGURABLE_FACTORS.map((factor) => factor.method).filter(
            (candidate) => candidate === method || current.includes(candidate),
          ),
    );
  }, []);

  const handleSubmit = useCallback(() => {
    // The fallback configures email on its own, so it satisfies the one-factor minimum.
    if (selected.length === 0 && !emailFallback) {
      setError('Select at least one method');
      return;
    }
    useConfigureMfaStore.getState().selectMethods(selected);
  }, [selected, emailFallback]);

  return (
    <FullPage
      id="configure-select-methods-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Choose the methods to configure" />
      <p className="description">
        <span>{`Pick the multi-factor authentication methods you want to set up.`}</span>
        <span>{`Methods already configured on your account are listed for reference.`}</span>
      </p>
      <div className="methods">
        {MFA_CONFIGURABLE_FACTORS.map(({ method }) => {
          // Not a pick the user can drop, the fallback registers email as it verifies.
          const isRequiredEmail = emailFallback && method === MfaMethod.Email;
          if (isRequiredEmail) {
            return (
              <TooltipProvider key={method} placement="right">
                <TooltipTrigger>
                  <div className="method-row">
                    <Checkbox text={mfaToText(method)} active disabled />
                  </div>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{EMAIL_REQUIRED_TOOLTIP}</p>
                </TooltipContent>
              </TooltipProvider>
            );
          }
          if (!isMfaFactorOfferable(method, configuredMethods)) {
            return (
              <div className="method-row configured" key={method}>
                <p className="name">{mfaToText(method)}</p>
                <span className="tag">Configured</span>
              </div>
            );
          }
          return (
            <div className="method-row" key={method}>
              <Checkbox
                text={mfaToText(method)}
                active={selected.includes(method)}
                onClick={() => {
                  toggle(method);
                }}
              />
            </div>
          );
        })}
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
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
