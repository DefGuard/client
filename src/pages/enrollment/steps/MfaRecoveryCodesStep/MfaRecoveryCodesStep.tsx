import './style.scss';
import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import {
  MessageBoxStyleVariant,
  MessageBoxType,
} from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import SvgIconCopy from '../../../../shared/defguard-ui/components/svg/IconCopy';
import { useClipboard } from '../../../../shared/hooks/useClipboard';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { EnrollmentStepKey } from '../../const';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { EnrollmentNavDirection } from '../../hooks/types';

export const MfaRecoveryCodesStep = () => {
  const { writeToClipboard } = useClipboard();
  const [setStore, navSubject] = useEnrollmentStore(
    (s) => [s.setState, s.nextSubject],
    shallow,
  );
  const codes = useEnrollmentStore((s) => s.recoveryCodes);

  // biome-ignore lint/correctness/useExhaustiveDependencies: rxjs
  useEffect(() => {
    const sub = navSubject.subscribe((dir) => {
      if (dir === EnrollmentNavDirection.NEXT) {
        setStore({ step: EnrollmentStepKey.ACTIVATE_USER });
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [navSubject]);

  return (
    <Card id="enrollment-mfa-recovery-codes-step">
      <div>
        <EnrollmentStepIndicator />
        <h3>Recovery codes</h3>
      </div>
      <MessageBox
        styleVariant={MessageBoxStyleVariant.FILLED}
        type={MessageBoxType.WARNING}
        message="Treat your recovery codes with the same level of attention as you would your password! We recommend saving them with a password manager such as Lastpass, 1Password, or Keeper."
      />
      <div className="codes">
        <ul>
          {codes.map((code) => (
            <li key={code}>{code}</li>
          ))}
        </ul>
        <Button
          text="Copy recovery codes"
          icon={<SvgIconCopy />}
          onClick={() => {
            writeToClipboard(codes.join('\n'));
          }}
        />
      </div>
    </Card>
  );
};
