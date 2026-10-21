import './style.scss';
import type { ReactNode } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { CodeInput } from '../../../../../shared/components/CodeInput/CodeInput';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Icon, type IconKindValue } from '../../../../../shared/components/Icon';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { isPresent } from '../../../../../shared/utils/isPresent';

const codeLength = 6;

interface Props {
  id: string;
  icon: IconKindValue;
  title: string;
  description: string[];
  code: string | null;
  error: string | null;
  onCodeChange: (value: string) => void;
  onSubmit: (pastedCode?: string) => void;
  leaveText: string;
  onLeave: () => void;
  extraAction?: ReactNode;
  codeLoading?: boolean;
  isLeaving?: boolean;
  leaveDisabled?: boolean;
  submitLoading?: boolean;
  submitDisabled?: boolean;
}

export const MfaCodeVerification = ({
  id,
  icon,
  title,
  description,
  code,
  error,
  onCodeChange,
  onSubmit,
  leaveText,
  onLeave,
  extraAction,
  codeLoading = false,
  isLeaving = false,
  leaveDisabled = false,
  submitLoading = false,
  submitDisabled = false,
}: Props) => {
  return (
    <FullPage id={id} className="mfa-code-verification" hideScrollContainer withControls>
      <div className="content">
        <Icon icon={icon} size={28} />
        <p className="title">{title}</p>
        <p className="description">
          {description.map((line) => (
            <span key={line}>{line}</span>
          ))}
        </p>
        <CodeInput
          length={codeLength}
          value={code}
          onChange={onCodeChange}
          error={error}
          onSubmit={onSubmit}
          loading={codeLoading}
        />
      </div>
      <Controls>
        <Button
          text={leaveText}
          variant={ButtonVariant.Secondary}
          loading={isLeaving}
          disabled={leaveDisabled}
          onClick={onLeave}
        />
        <div className="right">
          {isPresent(extraAction) && extraAction}
          <Button
            text="Continue"
            variant={ButtonVariant.Primary}
            loading={submitLoading}
            disabled={submitDisabled}
            onClick={() => {
              onSubmit();
            }}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
