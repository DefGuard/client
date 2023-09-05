import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';

export const LocationsList = () => {
  const { LL } = useI18nContext();
  const componentLL = LL.pages.client.locationsList;
  return (
    <section id="client-locations-list">
      <header>
        <h3>{componentLL.title()}</h3>
      </header>
    </section>
  );
};
