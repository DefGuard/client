import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import classNames from 'classnames';
import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useApi } from '../../../../shared/hooks/api/useApi';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { ConfigureDeviceCard } from './components/ConfigureDeviceCard/ConfigureDeviceCard';
import { QuickGuideCard } from './components/QuickGuideCard/QuickGuideCard';

export const DeviceStep = () => {
  const {
    enrollment: { activateUser },
  } = useApi();
  const { LL } = useI18nContext();
  const setStore = useEnrollmentStore((state) => state.setState);
  const deviceState = useEnrollmentStore((state) => state.deviceState);
  const vpnOptional = useEnrollmentStore((state) => state.vpnOptional);
  const [userPhone, userPassword] = useEnrollmentStore(
    (state) => [state.userInfo?.phone_number, state.userPassword],
    shallow,
  );
  const [nextSubject, next] = useEnrollmentStore(
    (state) => [state.nextSubject, state.nextStep],
    shallow,
  );

  const cn = classNames({
    required: !vpnOptional,
    optional: vpnOptional,
  });

  const { mutate } = useMutation({
    mutationFn: activateUser,
    onSuccess: () => {
      setStore({ loading: false });
      next();
    },
    onError: (err: AxiosError) => {
      setStore({ loading: false });
      console.error(err.message);
    },
  });

  useEffect(() => {
    if (userPhone && userPassword) {
      const sub = nextSubject.subscribe(() => {
        if ((deviceState && deviceState.device && deviceState.configs) || vpnOptional) {
          setStore({
            loading: true,
          });
          mutate({
            password: userPassword,
            phone_number: userPhone,
          });
        }
      });

      return () => {
        sub.unsubscribe();
      };
    }
  }, [deviceState, nextSubject, vpnOptional, setStore, userPhone, userPassword, mutate]);

  return (
    <div id="enrollment-device-step" className={cn}>
      {vpnOptional && (
        <MessageBox
          type={MessageBoxType.WARNING}
          message={LL.pages.enrollment.steps.deviceSetup.optionalMessage()}
        />
      )}
      <div className="cards">
        <ConfigureDeviceCard />
        <QuickGuideCard />
      </div>
    </div>
  );
};
