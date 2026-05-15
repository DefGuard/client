import './style.scss';
import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useEffect, useMemo } from 'react';
import { api } from '../../../../rust-api/api';
import { LocationMfaMode } from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { mfaToText } from '../../../../utils/mfa';
import { BoxIcon } from '../../../BoxIcon/BoxIcon';
import { Divider } from '../../../Divider/Divider';
import { Icon, IconKind } from '../../../Icon';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { ConnectButton } from '../../components/ConnectButton/ConnectButton';
import { ConnectionChart } from '../../components/ConnectionChart/ConnectionChart';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const ConnectedView = () => {
  const { location, setView } = useLocationCardContext();

  // const { data: currentConnection } = useQuery({
  //   queryKey: ['locations', location.id, 'connection'],
  //   queryFn: () =>
  //     api.getActiveConnection({
  //       connectionType: location.connection_type,
  //       locationId: location.id,
  //     }),
  // });

  const { data: lastConnection } = useQuery({
    queryKey: ['locations', location.id, 'last-connect'],
    queryFn: () =>
      api.getLastConnection({
        connectionType: location.connection_type,
        locationId: location.id,
      }),
  });

  const lastConnectedText = useMemo(() => {
    if (!lastConnection) return 'Never';
    return dayjs.utc(lastConnection.end).local().format('DD MMM YYYY');
  }, [lastConnection]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side-effect
  useEffect(() => {
    if (!location.active) {
      setView(LocationCardViews.Default);
    }
  }, [location.active]);

  return (
    <div className="location-view-connected">
      <SizedBox height={ThemeSpacing.Md} />
      <div className="tiles">
        <div className="tile">
          <BoxIcon>
            <Icon icon={IconKind.Globe} />
          </BoxIcon>
          <p className="label">Allowed traffic</p>
          <p className="label-value">
            {location.route_all_traffic ? 'All traffic' : 'Predefined traffic'}
          </p>
        </div>
        {location.location_mfa_mode !== LocationMfaMode.Disabled && (
          <div className="tile">
            <BoxIcon>
              <Icon icon={IconKind.LockClosed} />
            </BoxIcon>
            <p className="label">Active MFA</p>
            <p className="label-value">{mfaToText(location.mfa_method ?? 'totp')}</p>
          </div>
        )}
      </div>
      <Divider spacing={ThemeSpacing.Xl} />
      <div className="connection-info">
        <div className="info">
          <div className="label">Last connected</div>
          <div className="label-value">{lastConnectedText}</div>
        </div>
        <div className="info">
          <div className="label">Assigned IP</div>
          <div className="label-value">{location.address}</div>
        </div>
      </div>
      <SizedBox height={ThemeSpacing.Xl} />
      <ConnectionChart
        locationId={location.id}
        connectionType={location.connection_type}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <ConnectButton />
    </div>
  );
};
