import './style.scss';
import { Fragment } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { Timer } from '../../../../../shared/components/Timer/Timer';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

interface Props {
  backText?: string;
  nextText?: string;
  disableBack?: boolean;
  loading?: boolean;
  onNext: () => void;
  onBack?: () => void;
}

export const EnrollmentControls = ({
  backText = 'Back',
  disableBack = false,
  nextText = 'Continue',
  loading,
  onNext,
  onBack,
}: Props) => {
  const deadline = useEnrollmentStore((s) => s.deadline);

  console.log(deadline);

  return (
    <Fragment>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Controls className="enroll-controls">
        {!disableBack && (
          <Button
            variant={ButtonVariant.Secondary}
            disabled={loading}
            text={backText}
            onClick={onBack}
          />
        )}
        <div className="right">
          <Button
            variant={ButtonVariant.Primary}
            text={nextText}
            loading={loading}
            onClick={onNext}
          />
        </div>
        <div className="timer-float">
          {isPresent(deadline) && <Timer deadline={deadline} />}
        </div>
      </Controls>
    </Fragment>
  );
};
