import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { useAppData } from '../../../shared/providers/AppDataContext';
import { getLocationsQueryOptions } from '../../../shared/rust-api/query';
import { useAppStore } from '../../../shared/store/useAppStore';
import { isPresent } from '../../../shared/utils/isPresent';
import { OverviewSelection } from './components/OverviewSelection/OverviewSelection';

export const OverviewPage = () => {
  const { instances, tunnels } = useAppData();
  const selection = useAppStore((s) => s.compactViewSelection);

  const queryInstanceId = useMemo(() => {
    if (!isPresent(selection)) return instances[0].id;
    if (selection.kind === 'instance') return selection.data.id;
    return selection.data.instance_id;
  }, [selection, instances]);

  const { data: locations } = useQuery(getLocationsQueryOptions(queryInstanceId));

  const _displayedLocations = useMemo(() => {
    if (!isPresent(selection) || selection.kind === 'instance') {
      return locations ?? [];
    }
    return [selection.data];
  }, [selection, locations]);

  return (
    <FullPage id="overview-page" hideScrollContainer>
      <div className="page-grid">
        <OverviewSelection instances={instances} tunnels={tunnels} />
        <div className="overview-content"></div>
      </div>
    </FullPage>
  );
};
