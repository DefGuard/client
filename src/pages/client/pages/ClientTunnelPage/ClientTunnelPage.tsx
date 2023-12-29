import './style.scss';

import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { routes } from '../../../../shared/routes';
import { useClientStore } from '../../hooks/useClientStore';
import { LocationsList } from '../ClientInstancePage/components/LocationsList/LocationsList';
import { StatsFilterSelect } from '../ClientInstancePage/components/StatsFilterSelect/StatsFilterSelect';
import { DeleteInstanceModal } from '../ClientInstancePage/modals/DeleteInstanceModal/DeleteInstanceModal';
import { UpdateInstanceModal } from '../ClientInstancePage/modals/UpdateInstanceModal/UpdateInstanceModal';

export const ClientTunnelPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.tunnelPage;
  const tunnels = useClientStore((state) => state.tunnels);
  const selectedInstanceId = useClientStore((state) => state.selectedInstance);
  const selectedInstance = useMemo(
    () => tunnels.find((i) => i.id === selectedInstanceId),
    [tunnels, selectedInstanceId],
  );
  const navigate = useNavigate();

  // router guard, if no tunnels redirect to add instance, for now, later this will be replaced by init welcome flow
  useEffect(() => {
    if (tunnels.length === 0) {
      navigate(routes.client.addTunnel, { replace: true });
    }
  }, [tunnels, navigate]);

  return (
    <section id="client-instance-page" className="client-page">
      <header>
        <h1>{pageLL.title()}</h1>
        <div className="options">
          <StatsFilterSelect />
          <Button
            styleVariant={ButtonStyleVariant.STANDARD}
            text={LL.pages.client.pages.instancePage.header.edit()}
            disabled={!selectedInstance}
          />
        </div>
      </header>
      <LocationsList />
      <UpdateInstanceModal />
      <DeleteInstanceModal />
    </section>
  );
};
