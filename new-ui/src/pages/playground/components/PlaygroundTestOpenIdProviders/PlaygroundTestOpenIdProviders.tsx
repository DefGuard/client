import './style.scss';
import { ExternalProviderButton } from '../../../../shared/components/ExternalProviderButton/ExternalProviderButton';
import { MfaSelector } from '../../../../shared/components/LocationCard/components/MfaSelector/MfaSelector';
import { type InstanceInfo, MfaMethod } from '../../../../shared/rust-api/types';
import { findOpenIdProvider, openIdProviderName } from '../../../../shared/utils/mfa';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const noop = () => {};

const instances: Pick<InstanceInfo, 'openid_display_name' | 'openid_provider_kind'>[] = [
  { openid_display_name: 'Microsoft', openid_provider_kind: 'microsoft' },
  { openid_display_name: 'Google', openid_provider_kind: 'google' },
  { openid_display_name: 'Okta', openid_provider_kind: 'okta' },
  { openid_display_name: 'JumpCloud', openid_provider_kind: 'jumpcloud' },
  { openid_display_name: 'Keycloak', openid_provider_kind: 'custom' },
  {
    openid_display_name: 'Contoso Enterprise Single Sign-On Identity Provider',
    openid_provider_kind: 'custom',
  },
];

export const PlaygroundTestOpenIdProviders = () => {
  return (
    <PlaygroundCard>
      <div className="playground-test-openid-providers">
        <h3>Authenticate (full view)</h3>
        <div className="track full">
          {instances.map((instance) => (
            <ExternalProviderButton
              key={instance.openid_display_name}
              text={`Authenticate with ${openIdProviderName(instance)}`}
              provider={findOpenIdProvider(instance) ?? 'custom'}
              onClick={noop}
            />
          ))}
        </div>
        <h3>Authenticate (tray view)</h3>
        <div className="track tray">
          {instances.map((instance) => (
            <ExternalProviderButton
              key={instance.openid_display_name}
              text={`Authenticate with ${openIdProviderName(instance)}`}
              provider={findOpenIdProvider(instance) ?? 'custom'}
              onClick={noop}
            />
          ))}
        </div>
        <h3>Authenticate (loading)</h3>
        <div className="track full">
          <ExternalProviderButton
            text="Authenticate with Google"
            provider="google"
            loading
            onClick={noop}
          />
        </div>
        <h3>MFA method list</h3>
        <div className="track">
          {instances.map((instance) => (
            <MfaSelector
              key={instance.openid_display_name}
              factor={MfaMethod.Oidc}
              instance={instance}
              isSelectable
            />
          ))}
        </div>
        <h3>MFA method list (selected)</h3>
        <div className="track">
          {instances.map((instance) => (
            <MfaSelector
              key={instance.openid_display_name}
              factor={MfaMethod.Oidc}
              instance={instance}
              isSelectable
              selected
            />
          ))}
        </div>
        <h3>MFA method list (not configured)</h3>
        <div className="track">
          {instances.map((instance) => (
            <MfaSelector
              key={instance.openid_display_name}
              factor={MfaMethod.Oidc}
              instance={instance}
              configured={false}
              isSelectable={false}
            />
          ))}
        </div>
      </div>
    </PlaygroundCard>
  );
};
