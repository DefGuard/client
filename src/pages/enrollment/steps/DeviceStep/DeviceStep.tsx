import './style.scss';

import classNames from 'classnames';
import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { DesktopSetup } from './components/DesktopSetup/DesktopSetup';

export const DeviceStep = () => {
  const deviceName = useEnrollmentStore((state) => state.deviceName);
  const vpnOptional = useEnrollmentStore((state) => state.vpnOptional);
  const [nextSubject, next] = useEnrollmentStore(
    (state) => [state.nextSubject, state.nextStep],
    shallow,
  );

  const cn = classNames({
    required: !vpnOptional,
    optional: vpnOptional,
  });

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      if (deviceName) {
        next();
      }
    });

    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject, next, deviceName]);

  return (
    <div id="enrollment-device-step" className={cn}>
      <div className="cards">
        <DesktopSetup />
      </div>
    </div>
  );
};
