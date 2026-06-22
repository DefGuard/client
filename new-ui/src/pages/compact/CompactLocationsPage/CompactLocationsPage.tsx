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
import { useAppData } from '../../../shared/providers/AppDataContext';
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
  const { viewSelection: selection, setViewSelection } = useAppData();
  const openLocation = useAppStore((s) => s.expandedLocation);

  const routeData = useLoaderData({ from: '/compact/' });

  const { data: instances } = useQuery(getInstancesQueryOptions);

  const allInstances = instances ?? routeData.instances;
  const allTunnels = routeData.tunnels;

  const queryInstanceId = useMemo(() => {
    if (!isPresent(selection)) return routeData.instances[0].id;
    if (selection.kind === 'instance') return selection.id;
    return (
      allTunnels.find((t) => t.id === selection.id)?.instance_id ??
      routeData.instances[0].id
    );
  }, [selection, routeData.instances, allTunnels]);

  const { data: locations } = useQuery(getLocationsQueryOptions(queryInstanceId));

  const instanceInfo = useMemo(() => {
    if (!isPresent(selection)) return allInstances[0];
    if (selection.kind === 'instance')
      return allInstances.find((i) => i.id === selection.id);
    const tunnel = allTunnels.find((t) => t.id === selection.id);
    return tunnel ? allInstances.find((i) => i.id === tunnel.instance_id) : undefined;
  }, [selection, allInstances, allTunnels]);

  const displayedLocations = useMemo(() => {
    if (!isPresent(selection) || selection.kind === 'instance') {
      return locations ?? routeData.locations;
    }
    const tunnel = allTunnels.find((t) => t.id === selection.id);
    return tunnel ? [tunnel] : [];
  }, [selection, locations, routeData.locations, allTunnels]);

  useEffect(() => {
    if (selection?.kind === 'tunnel') return;
    if (selection === null || instanceInfo === undefined) {
      setViewSelection({ kind: 'instance', id: routeData.instances[0].id });
    }
  }, [routeData.instances, instanceInfo, selection, setViewSelection]);

  return (
    <CompactPage
      containerProps={{
        id: 'compact-locations-page',
      }}
    >
      <WindowHeader variant="compact" />
      <ScrollContainer>
        <div className="main-content">
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
