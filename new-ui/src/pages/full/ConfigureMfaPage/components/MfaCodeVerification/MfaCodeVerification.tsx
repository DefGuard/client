import './style.scss';
import type { PropsWithChildren, ReactNode } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Icon, type IconKindValue } from '../../../../../shared/components/Icon';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { isPresent } from '../../../../../shared/utils/isPresent';

interface Props extends PropsWithChildren {
  id: string;
  icon: IconKindValue;
  title: string;
  description: string[];
  onSubmit: () => void;
  submitText?: string;
  leaveText: string;
  onLeave: () => void;
  extraAction?: ReactNode;
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
  children,
  onSubmit,
  submitText = 'Continue',
  leaveText,
  onLeave,
  extraAction,
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
        {children}
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
            text={submitText}
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
