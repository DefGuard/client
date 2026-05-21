import './style.scss';
import { ThemeSpacing } from '../../../../types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
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

  const goToDefault = () => {
    setPostureError(null);
    setView(LocationCardViews.Default);
  };

  const tryAgain = () => {
    setPostureError(null);
    setView(retryView);
  };

  return (
    <div className="location-card-posture-check-fail-view">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title="Posture check failed">
        <p className="error">
          {postureError ?? 'Your device did not pass posture check.'}
        </p>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={goToDefault}
        />
        <div className="right">
          <Button text="Try again" variant={ButtonVariant.Primary} onClick={tryAgain} />
        </div>
      </Controls>
    </div>
  );
};
