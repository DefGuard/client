import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useLoaderData } from '@tanstack/react-router';
import { useEffect, useMemo } from 'react';
import { Divider } from '../../../shared/components/Divider/Divider';
import { LocationCard } from '../../../shared/components/LocationCard/LocationCard';
import { WindowHeader } from '../../../shared/components/WindowHeader/WindowHeader';
import {
  getInstancesQueryOptions,
  getLocationsQueryOptions,
} from '../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { CompactPage } from '../CompactPage/CompactPage';
import { useCompactLocationStore } from './hooks/useCompactLocationsStore';

export const CompactLocationsPage = () => {
  const selectedInstanceId = useCompactLocationStore((s) => s.selectedInstance);
  const openLocation = useCompactLocationStore((s) => s.expandedLocation);

  const routeData = useLoaderData({ from: '/' });

  const { data: locations } = useQuery(
    getLocationsQueryOptions(selectedInstanceId ?? routeData.instances[0].id),
  );

  const { data: instances } = useQuery(getInstancesQueryOptions);

  const instanceInfo = useMemo(
    () => (instances ?? routeData.instances).find((i) => i.id === selectedInstanceId),
    [selectedInstanceId, instances, routeData.instances],
  );

  useEffect(() => {
    if (selectedInstanceId === null || instanceInfo === null) {
      useCompactLocationStore.setState({
        selectedInstance: routeData.instances[0].id,
      });
    }
  }, [routeData.instances[0].id, instanceInfo, selectedInstanceId]);

  return (
    <CompactPage
      containerProps={{
        id: 'compact-locations-page',
      }}
    >
      <WindowHeader variant="compact" />
      <div className="locations">
        {isPresent(instanceInfo) &&
          (locations ?? routeData.locations).map((location) => {
            const isOpen = location.id === openLocation;
            return (
              <LocationCard
                instance={instanceInfo}
                disableOpen={(locations?.length ?? 0) <= 1}
                location={location}
                key={`${location.instance_id}-${location.id}`}
                isOpen={isOpen}
                onOpen={() => {
                  if (isOpen) {
                    useCompactLocationStore.setState({ expandedLocation: null });
                  } else {
                    useCompactLocationStore.setState({ expandedLocation: location.id });
                  }
                }}
              />
            );
          })}
      </div>
      <div className="compact-footer">
        <Divider spacing={ThemeSpacing.Md} />
      </div>
    </CompactPage>
  );
};
