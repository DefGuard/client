import './style.scss';

import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { routes } from '../../../../shared/routes';
import { useClientStore } from '../../hooks/useClientStore';
import { LocationsList } from '../ClientInstancePage/components/LocationsList/LocationsList';
import { StatsFilterSelect } from '../ClientInstancePage/components/StatsFilterSelect/StatsFilterSelect';

export const ClientTunnelPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.tunnelPage;
  const tunnels = useClientStore((state) => state.tunnels);
  const navigate = useNavigate();

  // router guard, if no tunnels redirect to add tunnel
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
        </div>
      </header>
      <LocationsList />
    </section>
  );
};
