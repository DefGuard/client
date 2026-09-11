import './style.scss';
import { ThemeSpacing } from '../../../../types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Divider } from '../../../Divider/Divider';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { NoConnectionIcon } from '../../images/NoConnectionIcon';

export const LocationCardPostureCheckFailView = () => {
  const { postureError, setPostureError, setView } = useLocationCardContext();

  const backToLocation = () => {
    setPostureError(null);
    setView(LocationCardViews.Default);
  };

  const postureErrors = postureError
    ? postureError
        .split(',')
        .map((error) => error.trim())
        .filter(Boolean)
    : ['Your device did not pass posture check.'];

  return (
    <div className="location-card-posture-check-fail-view">
      <Divider spacing={ThemeSpacing.Md} />
      <SizedBox height={ThemeSpacing.Xl} />
      <NoConnectionIcon />
      <SizedBox height={ThemeSpacing.Xl} />
      <LocationViewHeader title="Posture check failed">
        <div className="posture-errors">
          {postureErrors.map((error) => (
            <p className="error" key={error}>
              {error}
            </p>
          ))}
        </div>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <Button text="Got it" variant={ButtonVariant.Secondary} onClick={backToLocation} />
    </div>
  );
};
