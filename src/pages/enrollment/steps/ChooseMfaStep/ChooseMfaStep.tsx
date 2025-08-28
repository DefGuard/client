import './style.scss';

import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LabeledRadio } from '../../../../shared/defguard-ui/components/Layout/LabeledRadio/LabeledRadio';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MfaMethod } from '../../../../shared/types';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { EnrollmentStepKey } from '../../const';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { EnrollmentNavDirection } from '../../hooks/types';

export const ChooseMfaStep = () => {
  const selectedMethod = useEnrollmentStore((s) => s.mfaMethod);
  const [setStore, navSubject] = useEnrollmentStore(
    (s) => [s.setState, s.nextSubject],
    shallow,
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: rxjs
  useEffect(() => {
    const sub = navSubject.subscribe((dir) => {
      if (dir === EnrollmentNavDirection.NEXT) {
        setStore({ step: EnrollmentStepKey.MFA_SETUP });
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [navSubject]);

  return (
    <Card id="enrollment-choose-mfa-step">
      <div>
        <EnrollmentStepIndicator />
        <h3>Enable Multi-Factor Authentication</h3>
      </div>
      <MessageBox
        message={
          'To comply with the latest security standards and best practices, we require all users to configure at least one Multi-Factor Authentication (MFA) method for this defguard instance. Please configure one using this wizard. If you would like to configure more methods, edit your password and authentication section in your user profile settings.'
        }
      />
      <div className="choices">
        <LabeledRadio
          active={selectedMethod === MfaMethod.TOTP}
          label="Time Based One Time Passwords"
          onClick={() => {
            setStore({ mfaMethod: MfaMethod.TOTP });
          }}
        />
        <LabeledRadio
          active={selectedMethod === MfaMethod.EMAIL}
          label="Email"
          onClick={() => {
            setStore({ mfaMethod: MfaMethod.EMAIL });
          }}
        />
      </div>
      <div className="controls">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.LINK}
          text="Skip MFA Setup"
          onClick={() => {
            setStore({
              loading: false,
              step: EnrollmentStepKey.ACTIVATE_USER,
            });
          }}
        />
        <Button
          text="Setup selected MFA"
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          onClick={() => {
            setStore({
              step: EnrollmentStepKey.MFA_SETUP,
            });
          }}
        />
      </div>
    </Card>
  );
};
