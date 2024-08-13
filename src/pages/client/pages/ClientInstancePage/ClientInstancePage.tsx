import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { routes } from '../../../../shared/routes';
import { clientApi } from '../../clientAPI/clientApi';
import { useClientStore } from '../../hooks/useClientStore';
import { clientQueryKeys } from '../../query';
import { DefguardInstance, WireguardInstanceType } from '../../types';
import { LocationsList } from './components/LocationsList/LocationsList';
import { StatsFilterSelect } from './components/StatsFilterSelect/StatsFilterSelect';
import { StatsLayoutSelect } from './components/StatsLayoutSelect/StatsLayoutSelect';
import { DeleteInstanceModal } from './modals/DeleteInstanceModal/DeleteInstanceModal';
import { UpdateInstanceModal } from './modals/UpdateInstanceModal/UpdateInstanceModal';
import { useUpdateInstanceModal } from './modals/UpdateInstanceModal/useUpdateInstanceModal';

const { getLocations, getTunnels } = clientApi;

export const ClientInstancePage = () => {
  const { LL } = useI18nContext();
  const instanceLL = LL.pages.client.pages.instancePage;
  const tunnelLL = LL.pages.client.pages.tunnelPage;
  const instances = useClientStore((state) => state.instances);
  const tunnels = useClientStore((state) => state.tunnels);
  const [selectedInstanceId, selectedInstanceType] = useClientStore((state) => [
    state.selectedInstance?.id,
    state.selectedInstance?.type,
  ]);

  const selectedInstance = useMemo((): DefguardInstance | undefined => {
    if (
      !isUndefined(selectedInstanceId) &&
      selectedInstanceType &&
      selectedInstanceType === WireguardInstanceType.DEFGUARD_INSTANCE
    ) {
      return instances.find((i) => i.id === selectedInstanceId);
    }
  }, [selectedInstanceId, selectedInstanceType, instances]);

  const navigate = useNavigate();

  const isLocationPage = selectedInstanceType === WireguardInstanceType.DEFGUARD_INSTANCE;

  const openUpdateInstanceModal = useUpdateInstanceModal((state) => state.open);

  const queryKey = useMemo(() => {
    if (selectedInstanceType === WireguardInstanceType.DEFGUARD_INSTANCE) {
      return [clientQueryKeys.getLocations, selectedInstanceId as number];
    } else {
      return [clientQueryKeys.getTunnels];
    }
  }, [selectedInstanceId, selectedInstanceType]);

  const queryFn = useCallback(() => {
    if (selectedInstanceType === WireguardInstanceType.DEFGUARD_INSTANCE) {
      return getLocations({ instanceId: selectedInstanceId as number });
    } else {
      return getTunnels();
    }
  }, [selectedInstanceType, selectedInstanceId]);

  const { data: locations, isError } = useQuery({
    queryKey,
    queryFn,
    enabled: !!selectedInstance,
  });

  useEffect(() => {
    const isDefguardInstance =
      selectedInstanceType === WireguardInstanceType.DEFGUARD_INSTANCE;
    const isTunnelInstance = selectedInstanceType === WireguardInstanceType.TUNNEL;

    if (isDefguardInstance && !selectedInstance) {
      navigate(routes.client.addInstance, { replace: true });
    } else if (isTunnelInstance && tunnels.length === 0) {
      navigate(routes.client.addTunnel, { replace: true });
    }
  }, [selectedInstance, selectedInstanceType, tunnels.length, navigate]);

  return (
    <section id="client-instance-page" className="client-page">
      <header>
        <h1>{isLocationPage ? instanceLL.title() : tunnelLL.title()}</h1>
        <div className="options">
          <StatsFilterSelect />
          {isLocationPage && (
            <>
              <StatsLayoutSelect locations={locations} />
              {selectedInstance && (
                <Button
                  styleVariant={ButtonStyleVariant.STANDARD}
                  text={LL.pages.client.pages.instancePage.header.edit()}
                  disabled={!selectedInstance}
                  onClick={() => {
                    if (selectedInstance) {
                      openUpdateInstanceModal(selectedInstance);
                    }
                  }}
                />
              )}
            </>
          )}
          {!isLocationPage && selectedInstanceId && (
            <Button
              styleVariant={ButtonStyleVariant.STANDARD}
              text={LL.pages.client.pages.tunnelPage.header.edit()}
              disabled={!selectedInstanceId}
              onClick={() => {
                navigate(routes.client.editTunnel, { replace: true });
              }}
            />
          )}
        </div>
      </header>
      <LocationsList locations={locations} isError={isError} />
      <UpdateInstanceModal />
      <DeleteInstanceModal />
    </section>
  );
};
