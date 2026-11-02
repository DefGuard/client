import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Icon, IconKind } from '../../../../../shared/components/Icon';
import { MfaSelector } from '../../../../../shared/components/LocationCard/components/MfaSelector/MfaSelector';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { ConfigureFactorsSource } from '../../../../../shared/rust-api/types';
import { ThemeSpacing, ThemeVariable } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import '../style.scss';
import './style.scss';

interface Props {
  onCancel: () => void;
}

/** shown when the session offers more than one method, or only OpenID */
export const ConfigureSelectVerificationStep = ({ onCancel }: Props) => {
  const navigate = useNavigate();
  const methods = useConfigureMfaStore((s) => s.verificationMethods);
  const error = useConfigureMfaStore((s) => s.verificationError);
  const fromAddPage = useConfigureMfaStore(
    (s) => s.source === ConfigureFactorsSource.AddPage,
  );
  const instance = useConfigureMfaStore((s) => s.instance) ?? undefined;

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    // the source is read before the discard resets the store
    mutationFn: async () => {
      const { source } = useConfigureMfaStore.getState();
      await discardMfaConfiguration();
      return source;
    },
    onSettled: (source) => {
      if (source === ConfigureFactorsSource.AddPage) {
        void navigate({ to: '/full/add/mfa' });
        return;
      }
      onCancel();
    },
  });

  return (
    <FullPage
      id="configure-select-verification-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <div className="content">
        <Icon
          icon={IconKind.UserShield}
          size={28}
          staticColor={ThemeVariable.FgWhite50}
        />
        <SizedBox height={ThemeSpacing.Xl2} />
        <p className="title">Verify your identity</p>
        <SizedBox height={ThemeSpacing.Sm} />
        <p className="subtitle">
          To add a new MFA method, please select one of the available options to verify
          your identity.
        </p>
        <SizedBox height={ThemeSpacing.Xl2} />
        {isPresent(error) && <p className="error">{error}</p>}
        <div className="methods">
          {methods.map((method) => (
            <MfaSelector
              key={method}
              factor={method}
              instance={instance}
              isSelectable={!isCancelling}
              onClick={() => {
                useConfigureMfaStore.getState().selectVerificationMethod(method);
              }}
            />
          ))}
        </div>
      </div>
      <Controls>
        <Button
          text={fromAddPage ? 'Back' : 'Cancel'}
          variant={ButtonVariant.Secondary}
          loading={isCancelling}
          onClick={() => {
            cancel();
          }}
        />
      </Controls>
    </FullPage>
  );
};
