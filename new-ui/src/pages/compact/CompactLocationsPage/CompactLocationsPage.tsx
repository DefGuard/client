import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useLoaderData } from '@tanstack/react-router';
import { useEffect, useMemo } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Divider } from '../../../shared/components/Divider/Divider';
import { LocationCard } from '../../../shared/components/LocationCard/LocationCard';
import { ScrollContainer } from '../../../shared/components/ScrollContainer/ScrollContainer';
import { WindowHeader } from '../../../shared/components/WindowHeader/WindowHeader';
import { api } from '../../../shared/rust-api/api';
import {
  getInstancesQueryOptions,
  getLocationsQueryOptions,
} from '../../../shared/rust-api/query';
import { useAppStore } from '../../../shared/store/useAppStore';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { CompactPage } from '../CompactPage/CompactPage';
import { InstanceSwitcher } from './components/InstanceSwitcher';

export const CompactLocationsPage = () => {
  const selection = useAppStore((s) => s.compactViewSelection);
  const openLocation = useAppStore((s) => s.expandedLocation);

  const routeData = useLoaderData({ from: '/compact/' });

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
      useAppStore.setState({
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
      <ScrollContainer>
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
                      useAppStore.setState({ expandedLocation: null });
                    } else {
                      useAppStore.setState({ expandedLocation: location.id });
                    }
                  }}
                />
              );
            })}
        </div>
      </ScrollContainer>
      <div className="compact-footer">
        <Divider spacing={ThemeSpacing.Md} />
        <Controls>
          <Button
            variant={ButtonVariant.Secondary}
            size="primary"
            text="Open Defguard"
            onClick={() => {
              void api.swapToFullView();
            }}
          />
        </Controls>
      </div>
    </CompactPage>
  );
};
