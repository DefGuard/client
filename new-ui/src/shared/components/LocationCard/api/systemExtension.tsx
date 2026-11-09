import { getCurrentWindow } from '@tauri-apps/api/window';
import { WindowId } from '../../../consts';
import { useConfirmModal } from '../../../hooks/confirmModal/useConfirmModal';
import { api } from '../../../rust-api/api';
import { TauriEvent } from '../../../rust-api/types';
import { emitInFullView } from '../../../utils/emitInFullView';
import { ButtonVariant } from '../../Button/types';

export const openSystemExtensionModal = () => {
  useConfirmModal.getState().open({
    title: 'Enable Network Extension',
    content: (
      <div className="markdown-render">
        <p>
          Defguard needs permission to create VPN connections on your Mac. Enable its Network
          Extension in macOS System Settings to continue.
        </p>
        <h4>How to enable it:</h4>
        <ul>
          <li>Open System Settings.</li>
          <li>Go to General → Login Items &amp; Extensions.</li>
          <li>Find Defguard under Network Extensions and enable it.</li>
        </ul>
      </div>
    ),
    submitProps: {
      variant: ButtonVariant.Primary,
      text: 'Open System Settings',
    },
    onSubmit: async () => {
      await api.openSystemExtensionSettings();
    },
  });
};

export const showSystemExtensionModal = async () => {
  if (getCurrentWindow().label === WindowId.FullView) {
    openSystemExtensionModal();
  } else {
    await emitInFullView(TauriEvent.SystemExtensionRequired);
  }
};
