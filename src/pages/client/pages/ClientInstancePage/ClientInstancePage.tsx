import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { LocationsList } from './components/LocationsList/LocationsList';
import { StatsFilterSelect } from './components/StatsFilterSelect/StatsFilterSelect';
import { StatsLayoutSelect } from './components/StatsLayoutSelect/StatsLayoutSelect';

export const ClientInstancePage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.instancePage;

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
    </section>
  );
};
