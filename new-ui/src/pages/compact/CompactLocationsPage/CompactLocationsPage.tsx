import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useLoaderData } from '@tanstack/react-router';
import { useEffect, useMemo } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Divider } from '../../../shared/components/Divider/Divider';
import { LocationCard } from '../../../shared/components/LocationCard/LocationCard';
import { WindowHeader } from '../../../shared/components/WindowHeader/WindowHeader';
import { api } from '../../../shared/rust-api/api';
import {
  getInstancesQueryOptions,
  getLocationsQueryOptions,
} from '../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { CompactPage } from '../CompactPage/CompactPage';
import { InstanceSwitcher } from './components/InstanceSwitcher';
import { useCompactLocationStore } from './hooks/useCompactLocationsStore';

export const CompactLocationsPage = () => {
  const selection = useCompactLocationStore((s) => s.compactViewSelection);
  const openLocation = useCompactLocationStore((s) => s.expandedLocation);

  const routeData = useLoaderData({ from: '/' });

  const queryInstanceId = useMemo(() => {
    if (!isPresent(selection)) return routeData.instances[0].id;
    if (selection.kind === 'instance') return selection.data.id;
    return selection.data.instance_id;
  }, [selection, routeData.instances]);

  const { data: locations } = useQuery(getLocationsQueryOptions(queryInstanceId));

  const { data: instances } = useQuery(getInstancesQueryOptions);

  const instanceInfo = useMemo(() => {
    const allInstances = instances ?? routeData.instances;
    if (!isPresent(selection)) return allInstances[0];
    if (selection.kind === 'instance')
      return allInstances.find((i) => i.id === selection.data.id);
    return allInstances.find((i) => i.id === selection.data.instance_id);
  }, [selection, instances, routeData.instances]);

  const displayedLocations = useMemo(() => {
    if (!isPresent(selection) || selection.kind === 'instance') {
      return locations ?? routeData.locations;
    }
    return [selection.data];
  }, [selection, locations, routeData.locations]);

  useEffect(() => {
    if (selection === null || instanceInfo === undefined) {
      useCompactLocationStore.setState({
        compactViewSelection: { kind: 'instance', data: routeData.instances[0] },
      });
    }
  }, [routeData.instances, instanceInfo, selection]);

  return (
    <CompactPage
      containerProps={{
        id: 'compact-locations-page',
      }}
    >
      <WindowHeader variant="compact" />
      <div className="scroll-wrap">
        <InstanceSwitcher />
        <div className="locations">
          {isPresent(instanceInfo) &&
            displayedLocations.map((location) => {
              const isOpen =
                location.id === openLocation || displayedLocations.length === 1;
              return (
                <LocationCard
                  instance={instanceInfo}
                  disableOpen={displayedLocations.length <= 1}
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
      </div>
      <div className="compact-footer">
        <Divider spacing={ThemeSpacing.Md} />
        <Controls>
          <Button
            variant={ButtonVariant.Secondary}
            size="primary"
            text="Open Defguard"
            onClick={() => {
              void api.swapToOldUi();
            }}
          />
        </Controls>
      </div>
    </CompactPage>
  );
};
