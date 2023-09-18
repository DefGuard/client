import './style.scss';

import classNames from 'classnames';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import SvgIconCheckmarkSmall from '../../../../../../shared/components/svg/IconCheckmarkSmall';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import SvgIconX from '../../../../../../shared/defguard-ui/components/svg/IconX';
import { DefguardLocation } from '../../../../types';

type Props = {
  location: DefguardLocation;
};

export const LocationCardConnectButton = ({ location }: Props) => {
  const { LL } = useI18nContext();

  const cn = classNames('location-card-connect-button', {
    connected: location.connected,
  });

  return (
    <Button
      className={cn}
      icon={location.connected ? <SvgIconX /> : <SvgIconCheckmarkSmall />}
      size={ButtonSize.SMALL}
      styleVariant={ButtonStyleVariant.STANDARD}
      text={
        location.connected
          ? LL.pages.client.controls.disconnect()
          : LL.pages.client.controls.connect()
      }
    />
  );
};
