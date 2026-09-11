import './style.scss';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useCallback, useMemo, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { ButtonMenu } from '../../../../../shared/components/ButtonMenu/MenuButton';
import { Checkbox } from '../../../../../shared/components/Checkbox/Checkbox';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../../shared/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../../../shared/types';
import { downloadText } from '../../../../../shared/utils/download';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const RecoveryCodesStep = () => {
  const codes = useEnrollmentStore((s) => s.userRecoveryCodes) ?? [];
  const codesActionValue = useMemo(() => codes.join('\n'), [codes]);
  const [confirmed, setConfirmed] = useState(false);
  const handleCopy = useCallback(async () => {
    writeText(codesActionValue).then(() => {
      Snackbar.default('Codes copied to clipboard');
    });
  }, [codesActionValue]);

  const handleDownload = useCallback(() => {
    downloadText(codesActionValue, `recovery`, 'txt').then(() => {
      Snackbar.default('Codes downloaded');
    });
  }, [codesActionValue]);

  return (
    <div id="recovery-codes-step" className="step-content">
      <header>
        <h1>Download recovery codes</h1>
        <p>{`Recovery codes are your backup access. Store them securely (e.g. in a password manager like LastPass or Bitwarden) in case you lose your authenticator app.`}</p>
      </header>
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="codes">
        {codes.map((code) => (
          <p key={code}>{code}</p>
        ))}
      </div>
      <SizedBox height={ThemeSpacing.Lg} />
      <div className="actions">
        <ButtonMenu
          variant={ButtonVariant.Outlined}
          text="Actions"
          menuItems={[
            {
              items: [
                { text: 'Download codes', icon: 'download', onClick: handleDownload },
                { text: 'Copy to Clipboard', icon: 'copy', onClick: handleCopy },
              ],
            },
          ]}
        />
      </div>
      <Controls>
        <Checkbox
          text="I have saved my codes"
          active={confirmed}
          onClick={() => {
            setConfirmed((s) => !s);
          }}
        />
        <div className="right">
          <Button
            text="Complete"
            variant={ButtonVariant.Primary}
            disabled={!confirmed}
            onClick={() => {
              useEnrollmentStore.getState().next();
            }}
          />
        </div>
      </Controls>
    </div>
  );
};
