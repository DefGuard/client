import './style.scss';
import clsx from 'clsx';
import type { ReactNode } from 'react';
import type { LocationInfo } from '../../rust-api/types';
import { Direction, ThemeSpacing } from '../../types';
import { Divider } from '../Divider/Divider';
import { Fold } from '../Fold/Fold';
import { IconKind } from '../Icon';
import { IconButton } from '../IconButton/IconButton';
import { IconButtonVariant } from '../IconButton/types';
import { LocationCardIcon } from './components/LocationCardIcon';
import { LocationCardProvider, useLocationCardContext } from './context/context';
import { LocationCardViews, type LocationCardViewsValue } from './context/types';
import { DefaultView } from './views/DefaultView/DefaultView';
import { LocationCardMfaEmailView } from './views/LocationCardMfaEmailView/LocationCardMfaEmailView';
import { LocationCardMfaSettings } from './views/LocationCardMfaSettings/LocationCardMfaSettings';
import { LocationCardMfaTotpView } from './views/LocationCardMfaTotpView/LocationCardMfaTotpView';

interface Props {
  location: LocationInfo;
  isOpen: boolean;
  onOpen: () => void;
  disableOpen?: boolean;
}

const views: Record<LocationCardViewsValue, ReactNode> = {
  [LocationCardViews.Default]: <DefaultView />,
  [LocationCardViews.MfaTotp]: <LocationCardMfaTotpView />,
  [LocationCardViews.MfaEmail]: <LocationCardMfaEmailView />,
  [LocationCardViews.MfaOidc]: null,
  [LocationCardViews.MfaMobile]: null,
  [LocationCardViews.MfaSettings]: <LocationCardMfaSettings />,
  [LocationCardViews.Connecting]: null,
  [LocationCardViews.Connected]: null,
  [LocationCardViews.PostureCheckFail]: null,
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
      <div className="top-track">
        <div className="left">
          <LocationCardIcon />
          <div className="info">
            <p className="label">Location</p>
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
              onClick={onOpen}
            />
          )}
        </div>
      </div>
      <Fold open={isOpen}>
        <Divider spacing={ThemeSpacing.Md} />
        {views[currentView]}
      </Fold>
    </div>
  );
};

export const LocationCard = ({
  location,
  isOpen,
  onOpen,
  disableOpen = false,
}: Props) => {
  return (
    <LocationCardProvider location={location}>
      <LocationCardInner isOpen={isOpen} onOpen={onOpen} disableOpen={disableOpen} />
    </LocationCardProvider>
  );
};
