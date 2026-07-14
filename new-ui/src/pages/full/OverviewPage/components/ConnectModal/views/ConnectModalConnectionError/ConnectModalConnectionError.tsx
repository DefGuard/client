import './style.scss';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { useConnectModal } from '../../hooks/useConnectModal';

export const ConnectModalConnectionError = () => {
  const close = () => useConnectModal.setState({ visible: false });

  return (
    <div id="connection-error-view">
      <p className="view-description">
        One or more external services are unavailable or unreachable. This may be caused
        by a network issue or a temporary service outage. Please try again later.
      </p>
      <Controls>
        <div className="right">
          <Button text="Got it" variant={ButtonVariant.Primary} onClick={close} />
        </div>
      </Controls>
    </div>
  );
};
