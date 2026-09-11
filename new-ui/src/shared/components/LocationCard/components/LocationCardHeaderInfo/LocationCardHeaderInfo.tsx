import './style.scss';
import { ConnectionType, type LocationInfo } from '../../../../rust-api/types';
import { ThemeVariable } from '../../../../types';
import { Icon, IconKind } from '../../../Icon';
import { LocationCardIcon } from '../LocationCardIcon';

interface Props {
  location: LocationInfo;
  onInfoClick?: () => void;
}

export const LocationCardHeaderInfo = ({ location, onInfoClick }: Props) => (
  <div className="location-card-header-info">
    <LocationCardIcon />
    <div className="info">
      <p className="label">
        {location.connection_type === ConnectionType.Location ? 'Location' : 'Tunnel'}
      </p>
      <div className="bottom">
        <p className="location-name">{location.name}</p>
        {onInfoClick && (
          <button
            type="button"
            className="info-btn"
            aria-label="Show location details"
            onClick={onInfoClick}
          >
            <Icon
              icon={IconKind.InfoOutlined}
              size={16}
              staticColor={ThemeVariable.FgWhite70}
            />
          </button>
        )}
        {location.active && (
          <div className="online-badge">
            <p>Online</p>
          </div>
        )}
      </div>
    </div>
  </div>
);
