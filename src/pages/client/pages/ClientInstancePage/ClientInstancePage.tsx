import './style.scss';

import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { routes } from '../../../../shared/routes';
import { useClientStore } from '../../hooks/useClientStore';
import { LocationsList } from './components/LocationsList/LocationsList';
import { StatsFilterSelect } from './components/StatsFilterSelect/StatsFilterSelect';
import { StatsLayoutSelect } from './components/StatsLayoutSelect/StatsLayoutSelect';
import { DeleteInstanceModal } from './modals/DeleteInstanceModal/DeleteInstanceModal';
import { UpdateInstanceModal } from './modals/UpdateInstanceModal/UpdateInstanceModal';
import { useUpdateInstanceModal } from './modals/UpdateInstanceModal/useUpdateInstanceModal';

export const ClientInstancePage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.instancePage;
  const instances = useClientStore((state) => state.instances);
  const selectedInstanceId = useClientStore((state) => state.selectedInstance?.id);
  const selectedInstance = useMemo(
    () => instances.find((i) => i.id === selectedInstanceId),
    [instances, selectedInstanceId],
  );
  const navigate = useNavigate();

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
        <h1>{pageLL.title()}</h1>
        <div className="options">
          <StatsFilterSelect />
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
        </div>
      </header>
      <LocationsList />
      <UpdateInstanceModal />
      <DeleteInstanceModal />
    </section>
  );
};
