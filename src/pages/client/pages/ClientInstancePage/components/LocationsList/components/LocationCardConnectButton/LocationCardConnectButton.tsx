import './style.scss';

import classNames from 'classnames';
import { useState } from 'react';
import { error } from 'tauri-plugin-log-api';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import SvgIconCheckmarkSmall from '../../../../../../../../shared/components/svg/IconCheckmarkSmall';
import { Button } from '../../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import SvgIconX from '../../../../../../../../shared/defguard-ui/components/svg/IconX';
import { useToaster } from '../../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { DefguardLocation } from '../../../../../../types';

const { connect, disconnect } = clientApi;

type Props = {
  location?: DefguardLocation;
  routeOption?: boolean;
};

export const LocationCardConnectButton = ({ location, routeOption = false }: Props) => {
  const toaster = useToaster();
  const [isLoading, setIsLoading] = useState(false);
  const { LL } = useI18nContext();

  const cn = classNames('location-card-connect-button', {
    connected: location?.active,
  });

  const handleClick = async () => {
    setIsLoading(true);
    try {
      if (location) {
        if (location?.active) {
          await disconnect({ locationId: location.id });
        } else {
          await connect({
            locationId: location?.id,
            useDefaultRoute: routeOption,
          });
        }
        setIsLoading(false);
      }
    } catch (e) {
      setIsLoading(false);
      toaster.error(LL.common.messages.error());
      error(`Error handling interface: ${e}`);
      console.error(e);
    }
  };

  return (
    <Button
      onClick={handleClick}
      className={cn}
      icon={location?.active ? <SvgIconX /> : <SvgIconCheckmarkSmall />}
      size={ButtonSize.SMALL}
      styleVariant={ButtonStyleVariant.STANDARD}
      loading={isLoading}
      text={
        location?.active
          ? LL.pages.client.pages.instancePage.controls.disconnect()
          : LL.pages.client.pages.instancePage.controls.connect()
      }
    />
  );
};
