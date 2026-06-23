import { useEffect } from 'react';
import { MfaMethod } from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { Divider } from '../../../Divider/Divider';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationCardConnectButton } from '../../components/LocationCardConnectButton';
import { LocationCardConnectionInfo } from '../../components/LocationCardConnectionInfo/LocationCardConnectionInfo';
import { LocationCardConnectionTiles } from '../../components/LocationCardConnectionTiles/LocationCardConnectionTiles';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const ConnectedView = () => {
  const { location, setView, setMfaMethod } = useLocationCardContext();

  // biome-ignore lint/correctness/useExhaustiveDependencies: side-effect
  useEffect(() => {
    if (!location.active) {
      setMfaMethod(location.mfa_method ?? MfaMethod.Totp);
      setView(LocationCardViews.Default);
    }
  }, [location.active]);

  return (
    <div className="location-view-connected">
      <SizedBox height={ThemeSpacing.Md} />
      <LocationCardConnectionTiles location={location} variant="compact" />
      <Divider spacing={ThemeSpacing.Xl} />
      <LocationCardConnectionInfo location={location} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <LocationCardConnectButton />
    </div>
  );
};
