import './style.scss';

import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { routes } from '../../../../shared/routes';
import { useClientStore } from '../../hooks/useClientStore';
import { WireguardInstanceType } from '../../types';
import { LocationsList } from './components/LocationsList/LocationsList';
import { StatsFilterSelect } from './components/StatsFilterSelect/StatsFilterSelect';
import { StatsLayoutSelect } from './components/StatsLayoutSelect/StatsLayoutSelect';
import { DeleteInstanceModal } from './modals/DeleteInstanceModal/DeleteInstanceModal';
import { UpdateInstanceModal } from './modals/UpdateInstanceModal/UpdateInstanceModal';
import { useUpdateInstanceModal } from './modals/UpdateInstanceModal/useUpdateInstanceModal';

export const ClientInstancePage = () => {
  const { LL } = useI18nContext();
  const instanceLL = LL.pages.client.pages.instancePage;
  const tunelLL = LL.pages.client.pages.tunnelPage;
  const instances = useClientStore((state) => state.instances);
  const [selectedInstanceId, selectedInstanceType] = useClientStore((state) => [
    state.selectedInstance?.id,
    state.selectedInstance?.type,
  ]);
  const selectedInstance = useMemo(
    () => instances.find((i) => i.id === selectedInstanceId),
    [instances, selectedInstanceId],
  );
  const navigate = useNavigate();

  const isLocationPage = selectedInstanceType === WireguardInstanceType.DEFGUARD_INSTANCE;

  const openUpdateInstanceModal = useUpdateInstanceModal((state) => state.open);

  // router guard, if no instances redirect to add instance, for now, later this will be replaced by init welcome flow
  useEffect(() => {
    if (instances.length === 0) {
      navigate(routes.client.addInstance, { replace: true });
    }
  }, [instances, navigate]);

  return (
    <section id="client-instance-page" className="client-page">
      <header>
        <h1>{isLocationPage ? instanceLL.title() : tunelLL.title()}</h1>
        <div className="options">
          <StatsFilterSelect />
          {isLocationPage && (
            <>
              <StatsLayoutSelect />
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
            </>
          )}
          {!isLocationPage && selectedInstanceId && (
            <Button
              styleVariant={ButtonStyleVariant.STANDARD}
              text={LL.pages.client.pages.tunnelPage.header.edit()}
              disabled={!selectedInstanceId}
              onClick={() => {
                if (selectedInstance) {
                  openUpdateInstanceModal(selectedInstance);
                }
              }}
            />
          )}
        </div>
      </header>
      <LocationsList />
      <UpdateInstanceModal />
      <DeleteInstanceModal />
    </section>
  );
};
