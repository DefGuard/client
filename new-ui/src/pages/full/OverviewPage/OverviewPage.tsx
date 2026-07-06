import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { platform } from '@tauri-apps/plugin-os';
import clsx from 'clsx';
import { Fragment, useMemo } from 'react';
import { OverviewLocationCard } from '../../../shared/components/OverviewLocationCard/OverviewLocationCard';
import { ScrollContainer } from '../../../shared/components/ScrollContainer/ScrollContainer';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { useAppData } from '../../../shared/providers/AppDataContext';
import { getLocationsQueryOptions } from '../../../shared/rust-api/query';
import type { InstanceInfo } from '../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { ConnectModal } from './components/ConnectModal/ConnectModal';
import { OverviewActionsButton } from './components/OverviewActionsButton/OverviewActionsButton';
import { OverviewSelection } from './components/OverviewSelection/OverviewSelection';
import { UpdateInstanceModal } from './components/UpdateInstanceModal/UpdateInstanceModal';
import { UpdateTunnelModal } from './components/UpdateTunnelModal/UpdateTunnelModal';

const isWindows = platform() === 'windows';

export const OverviewPage = () => {
  const { instances, tunnels } = useAppData();
  const { viewSelection: selection } = useAppData();

  const selectedTunnel = useMemo(
    () =>
      selection?.kind === 'tunnel'
        ? tunnels.find((t) => t.id === selection.id)
        : undefined,
    [selection, tunnels],
  );

  const selectedInstance = useMemo(
    () =>
      selection?.kind === 'instance'
        ? instances.find((i) => i.id === selection.id)
        : undefined,
    [selection, instances],
  );

  const queryInstanceId = useMemo(() => {
    if (!isPresent(selection)) return instances[0].id;
    if (selection.kind === 'instance') return selection.id;
    return selectedTunnel?.instance_id ?? instances[0].id;
  }, [selection, instances, selectedTunnel]);

  const { data: locations } = useQuery(getLocationsQueryOptions(queryInstanceId));

  const displayedLocations = useMemo(() => {
    if (!isPresent(selection) || selection.kind === 'instance') {
      return locations ?? [];
    }
    return selectedTunnel ? [selectedTunnel] : [];
  }, [selection, locations, selectedTunnel]);

  return (
    <Fragment>
      <FullPage id="overview-page" hideScrollContainer>
        <div className="page-grid">
          <OverviewSelection instances={instances} tunnels={tunnels} />
          <div
            className={clsx('overview-content', {
              windows: isWindows,
            })}
          >
            <div className="header">
              {selection?.kind === 'instance' && (
                <p>{`Locations (${displayedLocations.length})`}</p>
              )}
              <div className="right">
                <OverviewActionsButton
                  selection={selection}
                  instance={selectedInstance ?? null}
                />
              </div>
            </div>
            <SizedBox height={ThemeSpacing.Lg} />
            <ScrollContainer>
              <div className="locations">
                {displayedLocations.map((location) => {
                  const instance: InstanceInfo | undefined =
                    selection?.kind === 'instance' ? selectedInstance : undefined;
                  return (
                    <OverviewLocationCard
                      location={location}
                      instance={instance}
                      key={location.id}
                    />
                  );
                })}
              </div>
            </ScrollContainer>
          </div>
        </div>
      </FullPage>
      <ConnectModal />
      <UpdateTunnelModal />
      <UpdateInstanceModal />
    </Fragment>
  );
};
