import './style.scss';

import { isUndefined } from 'lodash-es';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { EnrollmentStepIndicator } from '../../../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { useEnrollmentStore } from '../../../../hooks/store/useEnrollmentStore';
import { CreateDevice } from './components/CreateDevice';
import { DeviceConfiguration } from './components/DeviceConfiguration/DeviceConfiguration';

export const ConfigureDeviceCard = () => {
  const { LL } = useI18nContext();

  const deviceState = useEnrollmentStore((state) => state.deviceState);

  const configAvailable =
    deviceState &&
    !isUndefined(deviceState?.device) &&
    !isUndefined(deviceState?.configs);

  const cardLL = LL.pages.enrollment.steps.deviceSetup.cards.device;

  return (
    <Card id="configure-device-card">
      <EnrollmentStepIndicator />
      <h3>{cardLL.title()}</h3>
      {!configAvailable && <CreateDevice />}
      {configAvailable && <DeviceConfiguration />}
    </Card>
  );
};
