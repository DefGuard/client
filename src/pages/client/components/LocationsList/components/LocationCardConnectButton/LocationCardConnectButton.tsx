import './style.scss';

import classNames from 'classnames';
import { useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import SvgIconCheckmarkSmall from '../../../../../../shared/components/svg/IconCheckmarkSmall';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import SvgIconX from '../../../../../../shared/defguard-ui/components/svg/IconX';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../clientAPI/clientApi';
import { DefguardLocation } from '../../../../types';

const { connect, disconnect } = clientApi;

type Props = {
  location: DefguardLocation;
};

export const LocationCardConnectButton = ({ location }: Props) => {
  const toaster = useToaster();
  const [isLoading, setIsLoading] = useState(false);
  const { LL } = useI18nContext();

  const active = false;

  const cn = classNames('location-card-connect-button', {
    connected: active,
  });

  const handleClick = async () => {
    setIsLoading(true);
    try {
      if (active) {
        await disconnect({ locationId: location.id });
      } else {
        await connect({ locationId: location.id });
      }
      setIsLoading(false);
    } catch (e) {
      setIsLoading(false);
      toaster.error(LL.common.messages.error());
      console.error(e);
    }
  };

  return (
    <Button
      onClick={handleClick}
      className={cn}
      icon={active ? <SvgIconX /> : <SvgIconCheckmarkSmall />}
      size={ButtonSize.SMALL}
      styleVariant={ButtonStyleVariant.STANDARD}
      loading={isLoading}
      text={
        active
          ? LL.pages.client.controls.disconnect()
          : LL.pages.client.controls.connect()
      }
    />
  );
};
