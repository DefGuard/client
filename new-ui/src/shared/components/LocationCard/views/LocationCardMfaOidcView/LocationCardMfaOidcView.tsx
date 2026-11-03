import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { getAppConfigQueryOptions } from '../../../../rust-api/query';
import { ThemeSpacing } from '../../../../types';
import { findOpenIdProvider, openIdProviderName } from '../../../../utils/mfa';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
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
  const { setView, autoConnectOpenid, stepLabel, instance, canPickOtherMethod } =
    useLocationCardContext();
  const providerName = openIdProviderName(instance);
  const showOtherMethods =
    canPickOtherMethod && appConfig?.auto_start_openid_mfa === false;
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
      <SizedBox height={ThemeSpacing.Xl} />
      <ExternalProviderButton
        className="provider"
        text={`Authenticate with ${providerName}`}
        provider={findOpenIdProvider(instance) ?? 'custom'}
        loading={isPolling || isStarting}
        onClick={() => void start()}
      />
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={() => setView(LocationCardViews.Default)}
        />
        {showOtherMethods && (
          <div className="right">
            <Button
              text="Other methods"
              variant={ButtonVariant.Outlined}
              onClick={() => {
                setView(LocationCardViews.MfaSettings);
              }}
            />
          </div>
        )}
      </Controls>
    </div>
  );
};
