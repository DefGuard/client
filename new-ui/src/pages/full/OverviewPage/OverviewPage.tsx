import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { platform } from '@tauri-apps/plugin-os';
import clsx from 'clsx';
import { Fragment, useEffect, useMemo, useState } from 'react';
import { Fold } from '../../../shared/components/Fold/Fold';
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
import { DetailsFold } from './components/DetailsFold/DetailsFold';
import { OverviewSelection } from './components/OverviewSelection/OverviewSelection';

const isWindows = platform() === 'windows';

export const OverviewPage = () => {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const { instances, tunnels } = useAppData();
  const { viewSelection: selection } = useAppData();

  const queryInstanceId = useMemo(() => {
    if (!isPresent(selection)) return instances[0].id;
    if (selection.kind === 'instance') return selection.data.id;
    return selection.data.instance_id;
  }, [selection, instances]);

  const { data: locations } = useQuery(getLocationsQueryOptions(queryInstanceId));

  const displayedLocations = useMemo(() => {
    if (!isPresent(selection) || selection.kind === 'instance') {
      return locations ?? [];
    }
    return [selection.data];
  }, [selection, locations]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect only relevant on selection
  useEffect(() => {
    if (selection?.kind === 'tunnel' && detailsOpen) {
      setDetailsOpen(false);
    }
  }, [selection?.kind]);

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
            <ScrollContainer>
              <div className="header">
                <p>{`Locations (${displayedLocations.length})`}</p>
                {/* {selection?.kind === 'instance' && (
                  <button
                    id="show-instance-details"
                    onClick={() => {
                      setDetailsOpen((s) => !s);
                    }}
                  >
                    <span>Show instance details</span>
                    <Icon
                      size={16}
                      icon={IconKind.ArrowSmall}
                      rotationDirection={detailsOpen ? Direction.DOWN : Direction.RIGHT}
                      staticColor={ThemeVariable.FgWhite80}
                    />
                  </button>
                )} */}
              </div>
              <div className="instance-details">
                <Fold open={detailsOpen && selection?.kind === 'instance'}>
                  <SizedBox height={ThemeSpacing.Xl} />
                  {selection?.kind === 'instance' && (
                    <DetailsFold data={selection.data} />
                  )}
                </Fold>
              </div>
              <SizedBox height={ThemeSpacing.Xl} />
              <div className="locations">
                {displayedLocations.map((location) => {
                  let instance: InstanceInfo | undefined;
                  if (selection?.kind === 'instance') {
                    instance = selection.data;
                  }
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
    </Fragment>
  );
};
