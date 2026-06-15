import './style.scss';
import { useMemo } from 'react';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { LocationViewHeader } from '../../../../../../../shared/components/LocationCard/components/LocationViewHeader/LocationViewHeader';
import { NoConnectionIcon } from '../../../../../../../shared/components/LocationCard/images/NoConnectionIcon';
import { SizedBox } from '../../../../../../../shared/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/types';
import { useConnectModal } from '../../hooks/useConnectModal';

export const ConnectModalPostureCheckFail = () => {
  const postureError = useConnectModal((s) => s.postureError);

  const postureErrors = useMemo(() => {
    if (!postureError) return ['Your device did not pass posture check.'];
    return postureError
      .split(',')
      .map((e) => e.trim())
      .filter(Boolean);
  }, [postureError]);

  const close = () => useConnectModal.setState({ visible: false });

  return (
    <div id="posture-check-fail-view">
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
      <Controls>
        <Button
          containerProps={{ className: 'full' }}
          text="Got it"
          variant={ButtonVariant.Secondary}
          onClick={close}
        />
      </Controls>
    </div>
  );
};
