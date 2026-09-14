import './style.scss';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useCallback, useMemo, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonSize, ButtonVariant } from '../../../../../shared/components/Button/types';
import { Checkbox } from '../../../../../shared/components/Checkbox/Checkbox';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { TooltipButton } from '../../../../../shared/components/TooltipButton/TooltipButton';
import { ThemeSpacing } from '../../../../../shared/types';
import { downloadText } from '../../../../../shared/utils/download';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';

/** Only reached for the account's first factor, Core issues recovery codes once. */
export const ConfigureRecoveryCodesStep = () => {
  const codes = useConfigureMfaStore((s) => s.recoveryCodes);
  const codesActionValue = useMemo(() => codes.join('\n'), [codes]);
  const [confirmed, setConfirmed] = useState(false);

  const handleCopy = useCallback(() => {
    void writeText(codesActionValue);
  }, [codesActionValue]);

  const handleDownload = useCallback(() => {
    void downloadText(codesActionValue, `recovery`, 'txt');
  }, [codesActionValue]);

  return (
    <div id="configure-recovery-codes-step" className="step-content">
      <header>
        <h1>Download recovery codes</h1>
        <p>{`Recovery codes are your backup access. Store them securely (e.g. in a password manager like LastPass or Bitwarden) in case you lose your authenticator app.`}</p>
      </header>
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="codes">
        <ul>
          {codes.map((code) => (
            <li key={code}>{code}</li>
          ))}
        </ul>
      </div>
      <SizedBox height={ThemeSpacing.Lg} />
      <div className="actions">
        <TooltipButton
          tooltipText="Codes downloaded"
          buttonProps={{
            text: 'Download codes',
            iconLeft: 'download',
            size: ButtonSize.Primary,
            variant: ButtonVariant.Outlined,
            onClick: handleDownload,
          }}
        />
        <TooltipButton
          tooltipText="Codes copied to clipboard"
          buttonProps={{
            text: 'Copy to Clipboard',
            iconLeft: 'copy',
            size: ButtonSize.Primary,
            variant: ButtonVariant.Outlined,
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
              useConfigureMfaStore.getState().next();
            }}
          />
        </div>
      </Controls>
    </div>
  );
};
