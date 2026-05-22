import './style.scss';
import { ThemeSpacing } from '../../../../types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Divider } from '../../../Divider/Divider';
import { Icon, IconKind } from '../../../Icon';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const LocationCardPostureCheckFailView = () => {
  const { postureError, previousView, setPostureError, setView } =
    useLocationCardContext();

  const retryView =
    previousView && previousView !== LocationCardViews.PostureCheckFail
      ? previousView
      : LocationCardViews.Default;

  const tryAgain = () => {
    setPostureError(null);
    setView(retryView);
  };

  return (
    <div className="location-card-posture-check-fail-view">
      <Divider spacing={ThemeSpacing.Md} />
      <Icon className="posture-warning-icon" icon={IconKind.WarningFilled} size={48} />
      <SizedBox height={ThemeSpacing.Md} />
      <LocationViewHeader title="Posture check failed">
        <p className="error">
          {postureError ?? 'Your device did not pass posture check.'}
        </p>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <Button text="Try again" variant={ButtonVariant.Secondary} onClick={tryAgain} />
    </div>
  );
};
