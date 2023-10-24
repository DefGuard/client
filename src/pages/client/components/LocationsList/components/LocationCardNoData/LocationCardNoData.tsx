import './style.scss';

import { useI18nContext } from '../../../../../../i18n/i18n-react';

export const LocationCardNoData = () => {
  const { LL } = useI18nContext();
  return (
    <div className="location-no-data">
      <p>{LL.pages.client.locationNoData.title()}</p>
      <p>{LL.pages.client.locationNoData.content()}</p>
    </div>
  );
};
