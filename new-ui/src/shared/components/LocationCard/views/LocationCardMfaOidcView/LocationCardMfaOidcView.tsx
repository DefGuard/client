import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../../../rust-api/api';
import { getAppConfigQueryOptions } from '../../../../rust-api/query';
import { ThemeSpacing } from '../../../../types';
import { findOpenIdProvider, openIdProviderName } from '../../../../utils/mfa';
import { Checkbox } from '../../../Checkbox/Checkbox';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { ExternalProviderButton } from '../../../ExternalProviderButton/ExternalProviderButton';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaOidcConnect } from '../../hooks/useMfaOidcConnect';

export const LocationCardMfaOidcView = () => {
  const { data: appConfig } = useQuery(getAppConfigQueryOptions);
  const { setView, autoConnectOpenid, stepLabel, instance } = useLocationCardContext();
  const providerName = openIdProviderName(instance);
  const { start, isStarting, isPolling } = useMfaOidcConnect(autoConnectOpenid);

  return (
    <div className="location-card-mfa-oidc">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title={stepLabel ?? 'Multi-factor authentication'}>
        {!isPolling && (
          <p>
            {`To connect to the VPN, sign in with ${providerName} by clicking button below. A browser window will open and redirect you to ${providerName} for authentication.`}
          </p>
        )}
        {isPolling && (
          <p>
            {`Complete the sign-in in your browser. This page will update automatically.`}
          </p>
        )}
      </LocationViewHeader>
      {!isPolling && !autoConnectOpenid && (
        <div className="actions">
          <SizedBox height={ThemeSpacing.Lg} />
          <Checkbox
            active={appConfig?.auto_start_openid_mfa}
            text={`Don't show this message again`}
            onClick={() => {
              void api.setAppConfig(
                {
                  auto_start_openid_mfa: !appConfig?.auto_start_openid_mfa,
                },
                true,
              );
            }}
          />
        </div>
      )}
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={() => setView(LocationCardViews.Default)}
        />
        <div className="right">
          <ExternalProviderButton
            text={`Authenticate with ${providerName}`}
            provider={findOpenIdProvider(instance) ?? 'custom'}
            loading={isPolling || isStarting}
            onClick={() => void start()}
          />
        </div>
      </Controls>
    </div>
  );
};
