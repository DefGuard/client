import './style.scss';
import { useState } from 'react';
import { Checkbox } from '../../../../shared/components/Checkbox/Checkbox';
import { MfaSettingsSection } from '../../../../shared/components/MfaSettingsSection/MfaSettingsSection';
import type {
  MfaSettingsInstance,
  MfaSettingsLocation,
} from '../../../../shared/components/MfaSettingsSection/types';
import { useMfaSettingsSection } from '../../../../shared/components/MfaSettingsSection/useMfaSettingsSection';
import { ConnectionType, MfaMethod } from '../../../../shared/rust-api/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const multiStepLocation: MfaSettingsLocation = {
  connection_type: ConnectionType.Location,
  mfa_step_plan: [],
  mfa_steps: [
    {
      methods: [
        { method: MfaMethod.Totp, configured: true },
        { method: MfaMethod.Email, configured: false },
        { method: MfaMethod.Biometric, configured: false },
      ],
    },
    {
      methods: [
        { method: MfaMethod.Oidc, configured: true },
        { method: MfaMethod.Totp, configured: true },
        { method: MfaMethod.Fido2, configured: true },
      ],
    },
    {
      methods: [
        { method: MfaMethod.Fido2, configured: true },
        { method: MfaMethod.MobileApprove, configured: false },
      ],
    },
  ],
};

const singleStepLocation: MfaSettingsLocation = {
  connection_type: ConnectionType.Location,
  mfa_step_plan: [],
  mfa_steps: [
    {
      methods: [
        { method: MfaMethod.Totp, configured: true },
        { method: MfaMethod.Oidc, configured: true },
        { method: MfaMethod.Email, configured: false },
      ],
    },
  ],
};

const instance: MfaSettingsInstance = {
  mfa_configured_methods: [MfaMethod.Totp, MfaMethod.Oidc, MfaMethod.Fido2],
};

const Demo = ({
  title,
  location,
  configurable,
}: {
  title: string;
  location: MfaSettingsLocation;
  configurable: boolean;
}) => {
  const mfaSection = useMfaSettingsSection({ location, instance, configurable });

  return (
    <div className="demo">
      <h3>{title}</h3>
      <MfaSettingsSection {...mfaSection.sectionProps} />
      <p>Plan: {mfaSection.plan.join(' → ') || 'none'}</p>
      <p>Configure MFA ({mfaSection.configureMethods.length})</p>
    </div>
  );
};

export const PlaygroundTestMfaSettingsSection = () => {
  const [configurable, setConfigurable] = useState(false);

  return (
    <PlaygroundCard>
      <div className="playground-test-mfa-settings-section">
        <Checkbox
          active={configurable}
          onClick={() => setConfigurable((current) => !current)}
          text="Configurable"
        />
        <div className="demos">
          <Demo
            title="Multi-step"
            location={multiStepLocation}
            configurable={configurable}
          />
          <Demo
            title="Single step"
            location={singleStepLocation}
            configurable={configurable}
          />
        </div>
      </div>
    </PlaygroundCard>
  );
};
