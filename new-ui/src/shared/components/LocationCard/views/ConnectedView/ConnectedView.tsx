import { ThemeSpacing } from '../../../../types';
import { Divider } from '../../../Divider/Divider';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationCardConnectButton } from '../../components/LocationCardConnectButton';
import { LocationCardConnectionInfo } from '../../components/LocationCardConnectionInfo/LocationCardConnectionInfo';
import { LocationCardConnectionTiles } from '../../components/LocationCardConnectionTiles/LocationCardConnectionTiles';
import { useLocationCardContext } from '../../context/context';

export const ConnectedView = () => {
  const { location, instance } = useLocationCardContext();

  return (
    <div className="location-view-connected">
      <SizedBox height={ThemeSpacing.Md} />
      <LocationCardConnectionTiles
        location={location}
        instance={instance}
        variant="compact"
      />
      <Divider spacing={ThemeSpacing.Xl} />
      <LocationCardConnectionInfo location={location} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <LocationCardConnectButton />
    </div>
  );
};
