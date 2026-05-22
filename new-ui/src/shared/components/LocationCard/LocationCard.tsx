import './style.scss';
import clsx from 'clsx';
import type { ReactNode } from 'react';
import {
  ConnectionType,
  type InstanceInfo,
  type LocationInfo,
} from '../../rust-api/types';
import { Direction } from '../../types';
import { Fold } from '../Fold/Fold';
import { IconKind } from '../Icon';
import { IconButton } from '../IconButton/IconButton';
import { IconButtonVariant } from '../IconButton/types';
import { LocationCardIcon } from './components/LocationCardIcon';
import { LocationCardProvider, useLocationCardContext } from './context/context';
import { LocationCardViews, type LocationCardViewsValue } from './context/types';
import { ConnectedView } from './views/ConnectedView/ConnectedView';
import { DefaultView } from './views/DefaultView/DefaultView';
import { LocationCardMfaEmailView } from './views/LocationCardMfaEmailView/LocationCardMfaEmailView';
import { LocationCardMfaMobileView } from './views/LocationCardMfaMobileView/LocationCardMfaMobileView';
import { LocationCardMfaOidcView } from './views/LocationCardMfaOidcView/LocationCardMfaOidcView';
import { LocationCardMfaSettings } from './views/LocationCardMfaSettings/LocationCardMfaSettings';
import { LocationCardMfaTotpView } from './views/LocationCardMfaTotpView/LocationCardMfaTotpView';
import { LocationCardPostureCheckFailView } from './views/LocationCardPostureCheckFailView/LocationCardPostureCheckFailView';

interface Props {
  location: LocationInfo;
  isOpen: boolean;
  onOpen: () => void;
  disableOpen?: boolean;
  instance: InstanceInfo;
}

const views: Record<LocationCardViewsValue, ReactNode> = {
  [LocationCardViews.Default]: <DefaultView />,
  [LocationCardViews.MfaTotp]: <LocationCardMfaTotpView />,
  [LocationCardViews.MfaEmail]: <LocationCardMfaEmailView />,
  [LocationCardViews.MfaOidc]: <LocationCardMfaOidcView />,
  [LocationCardViews.MfaMobile]: <LocationCardMfaMobileView />,
  [LocationCardViews.MfaSettings]: <LocationCardMfaSettings />,
  [LocationCardViews.Connecting]: null,
  [LocationCardViews.Connected]: <ConnectedView />,
  [LocationCardViews.PostureCheckFail]: <LocationCardPostureCheckFailView />,
};

interface InnerProps {
  isOpen: boolean;
  onOpen: () => void;
  disableOpen: boolean;
}

const LocationCardInner = ({ isOpen, onOpen, disableOpen }: InnerProps) => {
  const { location, currentView } = useLocationCardContext();

  return (
    <div
      className={clsx('location-card')}
      data-network={location.network_id}
      data-id={location.id}
    >
      <div
        className={clsx('top-track', {
          interactive: !disableOpen,
        })}
        onClick={onOpen}
      >
        <div className="left">
          <LocationCardIcon />
          <div className="info">
            <p className="label">
              {location.connection_type === ConnectionType.Location
                ? 'Location'
                : 'Tunnel'}
            </p>
            <div className="bottom">
              <p className="location-name">{location.name}</p>
              {location.active && (
                <div className="online-badge">
                  <p>Online</p>
                </div>
              )}
            </div>
          </div>
        </div>
        <div className="right">
          {!disableOpen && (
            <IconButton
              icon={IconKind.ArrowSmall}
              variant={isOpen ? IconButtonVariant.SmallSelected : IconButtonVariant.Small}
              iconRotation={isOpen ? Direction.DOWN : Direction.RIGHT}
            />
          )}
        </div>
      </div>
      <Fold open={isOpen}>{views[currentView]}</Fold>
    </div>
  );
};

export const LocationCard = ({
  location,
  isOpen,
  onOpen,
  instance,
  disableOpen = false,
}: Props) => {
  return (
    <LocationCardProvider location={location} instance={instance}>
      <LocationCardInner isOpen={isOpen} onOpen={onOpen} disableOpen={disableOpen} />
    </LocationCardProvider>
  );
};
