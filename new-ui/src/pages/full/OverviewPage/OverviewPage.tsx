import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { useAppData } from '../../../shared/providers/AppDataContext';
import { getLocationsQueryOptions } from '../../../shared/rust-api/query';
import { useAppStore } from '../../../shared/store/useAppStore';
import { isPresent } from '../../../shared/utils/isPresent';

export const OverviewPage = () => {
  const { instances } = useAppData();
  const selection = useAppStore((s) => s.compactViewSelection);

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

  return (
    <FullPage id="overview-page" hideScrollContainer>
      <div className="page-grid">
        <div className="selection">
          <div className="group">
            <p>{`Instances`}</p>
            <div className="items"></div>
          </div>
          <div className="group">
            <p>{`Tunnels`}</p>
            <div className="items"></div>
          </div>
        </div>
      </div>
      <div>
        <h2>Instances</h2>
        <ul>
          {instances.map((instance) => (
            <li key={instance.id}>{instance.name}</li>
          ))}
        </ul>
        <h2>Locations / Tunnels</h2>
        <ul>
          {displayedLocations.map((location) => (
            <li key={`${location.instance_id}-${location.id}`}>{location.name}</li>
          ))}
        </ul>
      </div>
    </FullPage>
  );
};
