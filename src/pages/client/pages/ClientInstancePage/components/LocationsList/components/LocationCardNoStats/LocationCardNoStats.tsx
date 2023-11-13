import './style.scss';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';

export const LocationCardNoStats = () => {
  const { LL } = useI18nContext();
  const componentLL = LL.pages.client.pages.instancePage.LocationNoStats;
  return (
    <div className="location-no-stats">
      <p>{componentLL.title()}</p>
      <p>{componentLL.content()}</p>
    </div>
  );
};
