import './TrayConnectionError.scss';
import { Button } from '../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../shared/components/Button/types';
import { Icon, IconKind } from '../../../../shared/components/Icon';
import { useTrayConnectionError } from '../../../../shared/store/useAppStore';

export const TrayConnectionError = () => {
  const close = () => useTrayConnectionError.setState({ visible: false });

  return (
    <div className="tray-connection-error">
      <Icon icon={IconKind.ServiceUnavailable} size={24} />
      <p className="description">
        One or more external services are unavailable or unreachable. This may be caused
        by a network issue or a temporary service outage. Please try again later.
      </p>
      <Button text="Back" variant={ButtonVariant.Primary} onClick={close} />
    </div>
  );
};
