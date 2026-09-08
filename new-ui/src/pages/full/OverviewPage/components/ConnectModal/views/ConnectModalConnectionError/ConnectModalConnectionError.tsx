import './style.scss';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { DEFAULT_CONNECTION_ERROR } from '../../../../../../../shared/components/LocationCard/api/connectError';
import { useConnectModal } from '../../hooks/useConnectModal';

export const ConnectModalConnectionError = () => {
  const close = () => useConnectModal.setState({ visible: false });
  const connectionError = useConnectModal((s) => s.connectionError);

  return (
    <div id="connection-error-view">
      <p className="view-description">{connectionError ?? DEFAULT_CONNECTION_ERROR}</p>
      <Controls>
        <div className="right">
          <Button text="Got it" variant={ButtonVariant.Primary} onClick={close} />
        </div>
      </Controls>
    </div>
  );
};
