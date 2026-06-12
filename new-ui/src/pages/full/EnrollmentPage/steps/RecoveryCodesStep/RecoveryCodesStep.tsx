import './style.scss';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useCallback, useMemo, useRef, useState } from 'react';
import { Subject } from 'rxjs';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Checkbox } from '../../../../../shared/components/Checkbox/Checkbox';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { TooltipButton } from '../../../../../shared/components/TooltipButton/TooltipButton';
import { ThemeSpacing } from '../../../../../shared/types';
import { downloadText } from '../../../../../shared/utils/download';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const RecoveryCodesStep = () => {
  const codes = useEnrollmentStore((s) => s.userRecoveryCodes) ?? [];
  const codesActionValue = useMemo(() => codes.join('\n'), [codes]);
  const [confirmed, setConfirmed] = useState(false);
  const clipboardSub = useRef(new Subject<void>());
  const downloadSub = useRef(new Subject<void>());

  const handleCopy = useCallback(async () => {
    writeText(codesActionValue).then(() => {
      clipboardSub.current.next();
    });
  }, [codesActionValue]);

  const handleDownload = useCallback(() => {
    downloadText(codesActionValue, `recovery`, 'txt').then(() => {
      downloadSub.current.next();
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
        <TooltipButton
          tooltipTrigger={downloadSub.current}
          tooltipText="Codes downloaded"
          buttonProps={{
            variant: ButtonVariant.Outlined,
            text: 'Download codes',
            iconLeft: 'download',
            onClick: handleDownload,
          }}
        />
        <TooltipButton
          tooltipTrigger={clipboardSub.current}
          tooltipText="Codes copied to clipboard"
          buttonProps={{
            variant: ButtonVariant.Outlined,
            text: 'Copy to Clipboard',
            iconLeft: 'copy',
            onClick: handleCopy,
          }}
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
