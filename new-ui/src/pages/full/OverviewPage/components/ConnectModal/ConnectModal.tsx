import './style.scss';

import { type ReactNode, useEffect } from 'react';
import { useShallow } from 'zustand/shallow';
import { Modal } from '../../../../../shared/components/Modal/Modal';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  ConnectModalTitle,
  ConnectModalView,
  type ConnectModalViewValue,
} from './hooks/types';
import { useConnectModal } from './hooks/useConnectModal';
import { ConnectModalMfaEmail } from './views/ConnectModalMfaEmail/ConnectModalMfaEmail';
import { ConnectModalMfaMobile } from './views/ConnectModalMfaMobile/ConnectModalMfaMobile';
import { ConnectModalMfaOidc } from './views/ConnectModalMfaOidc/ConnectModalMfaOidc';
import { ConnectModalMfaSettings } from './views/ConnectModalMfaSettings/ConnectModalMfaSettings';
import { ConnectModalMfaTotp } from './views/ConnectModalMfaTotp/ConnectModalMfaTotp';
import { ConnectModalPostureCheckFail } from './views/ConnectModalPostureCheckFail/ConnectModalPostureCheckFail';

export const ConnectModal = () => {
  const reset = useConnectModal((s) => s.reset);
  const [view, visible, location] = useConnectModal(
    useShallow((s) => [s.view, s.visible, s.location]),
  );
  const isOpen = isPresent(view) && isPresent(location) && visible;

  useEffect(() => {
    if (location?.active && visible) {
      useConnectModal.setState({
        visible: false,
      });
    }
  }, [location?.active, visible]);

  return (
    <Modal
      id="connect-modal"
      size="small"
      title={view ? ConnectModalTitle[view] : ''}
      isOpen={isOpen}
      afterClose={() => {
        reset();
      }}
      onClose={() => {
        useConnectModal.setState({
          visible: false,
        });
      }}
    >
      <ModalContent />
    </Modal>
  );
};

const viewContent: Record<ConnectModalViewValue, ReactNode> = {
  [ConnectModalView.MfaTotp]: <ConnectModalMfaTotp />,
  [ConnectModalView.MfaEmail]: <ConnectModalMfaEmail />,
  [ConnectModalView.MfaOidc]: <ConnectModalMfaOidc />,
  [ConnectModalView.MfaMobile]: <ConnectModalMfaMobile />,
  [ConnectModalView.MfaSettings]: <ConnectModalMfaSettings />,
  [ConnectModalView.PostureCheckFail]: <ConnectModalPostureCheckFail />,
} as const;

const ModalContent = () => {
  const activeView = useConnectModal((s) => s.view);
  if (!activeView) return null;

  return viewContent[activeView];
};
