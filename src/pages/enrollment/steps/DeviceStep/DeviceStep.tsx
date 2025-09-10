import './style.scss';

import classNames from 'classnames';
import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';
import { EnrollmentStepKey } from '../../const';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { EnrollmentNavDirection } from '../../hooks/types';
import { DesktopSetup } from './components/DesktopSetup/DesktopSetup';

export const DeviceStep = () => {
  const deviceName = useEnrollmentStore((state) => state.deviceName);
  const vpnOptional = useEnrollmentStore((state) => state.vpnOptional);

  const [nextSubject, setStore] = useEnrollmentStore(
    (state) => [state.nextSubject, state.setState],
    shallow,
  );

  const cn = classNames({
    required: !vpnOptional,
    optional: vpnOptional,
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: jsx
  useEffect(() => {
    const sub = nextSubject.subscribe((direction) => {
      switch (direction) {
        case EnrollmentNavDirection.BACK:
          setStore({ step: EnrollmentStepKey.PASSWORD });
          break;
        case EnrollmentNavDirection.NEXT:
          if (deviceName) {
            setStore({ step: EnrollmentStepKey.MFA_CHOICE });
          }
          break;
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject, deviceName]);

  return (
    <div id="enrollment-device-step" className={cn}>
      <div className="cards">
        <DesktopSetup />
      </div>
    </div>
  );
};
