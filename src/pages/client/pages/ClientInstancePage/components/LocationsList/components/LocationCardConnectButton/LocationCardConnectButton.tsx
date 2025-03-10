import './style.scss';

import { listen } from '@tauri-apps/api/event';
import classNames from 'classnames';
import { useEffect, useState } from 'react';
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
import { CommonWireguardFields } from '../../../../../../types';
import { useMFAModal } from '../../modals/MFAModal/useMFAModal';

const { connect, disconnect } = clientApi;

type Props = {
  location?: CommonWireguardFields;
};

type Payload = {
  location?: CommonWireguardFields;
};

export const LocationCardConnectButton = ({ location }: Props) => {
  const toaster = useToaster();
  const [isLoading, setIsLoading] = useState(false);
  const { LL } = useI18nContext();
  const openMFAModal = useMFAModal((state) => state.open);

  const cn = classNames('location-card-connect-button', {
    connected: location?.active,
  });

  const handleClick = async () => {
    setIsLoading(true);
    try {
      if (location) {
        if (location?.active) {
          await disconnect({
            locationId: location.id,
            connectionType: location.connection_type,
          });
        } else {
          if (location.mfa_enabled) {
            openMFAModal(location);
          } else {
            await connect({
              locationId: location?.id,
              connectionType: location.connection_type,
            });
          }
        }
        setIsLoading(false);
      }
    } catch (e) {
      setIsLoading(false);
      toaster.error(
        LL.common.messages.errorWithMessage({
          message: String(e),
        }),
      );
      error(`Error handling interface: ${e}`);
      console.error(e);
    }
  };

  useEffect(() => {
    async function listenMFAEvent() {
      await listen<Payload>('mfa-trigger', () => {
        if (location) {
          openMFAModal(location);
        }
      });
    }
    listenMFAEvent();
  }, [openMFAModal, location]);

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
