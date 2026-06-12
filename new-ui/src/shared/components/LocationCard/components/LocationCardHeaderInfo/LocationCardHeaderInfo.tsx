import './style.scss';
import { ConnectionType, type LocationInfo } from '../../../../rust-api/types';
import { LocationCardIcon } from '../LocationCardIcon';

interface Props {
  location: LocationInfo;
}

export const LocationCardHeaderInfo = ({ location }: Props) => (
  <div className="location-card-header-info">
    <LocationCardIcon />
    <div className="info">
      <p className="label">
        {location.connection_type === ConnectionType.Location ? 'Location' : 'Tunnel'}
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
);
