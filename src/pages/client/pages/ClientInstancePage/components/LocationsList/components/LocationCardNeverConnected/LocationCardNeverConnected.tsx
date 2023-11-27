import './style.scss';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';

export const LocationCardNeverConnected = () => {
  const { LL } = useI18nContext();
  const componentLL = LL.pages.client.pages.instancePage.locationNeverConnected;
  return (
    <div className="location-no-connections">
      <p>{componentLL.title()}</p>
      <p>{componentLL.content()}</p>
    </div>
  );
};
