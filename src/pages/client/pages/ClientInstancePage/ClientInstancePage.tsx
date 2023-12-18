import './style.scss';

import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { routes } from '../../../../shared/routes';
import { useClientStore } from '../../hooks/useClientStore';
import { LocationsList } from './components/LocationsList/LocationsList';
import { StatsFilterSelect } from './components/StatsFilterSelect/StatsFilterSelect';
import { StatsLayoutSelect } from './components/StatsLayoutSelect/StatsLayoutSelect';
import { DeleteInstanceModal } from './modals/DeleteInstanceModal/DeleteInstanceModal';
import { UpdateInstanceModal } from './modals/UpdateInstanceModal/UpdateInstanceModal';

export const ClientInstancePage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.instancePage;
  const instances = useClientStore((state) => state.instances);
  const navigate = useNavigate();

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
        </div>
      </header>
      <LocationsList />
      <UpdateInstanceModal />
      <DeleteInstanceModal />
    </section>
  );
};
