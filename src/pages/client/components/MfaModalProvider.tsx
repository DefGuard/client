import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { type PropsWithChildren, useEffect } from 'react';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { MFAModal } from '../pages/ClientInstancePage/components/LocationsList/modals/MFAModal/MFAModal';
import { useMFAModal } from '../pages/ClientInstancePage/components/LocationsList/modals/MFAModal/useMFAModal';
import { ClientConnectionType, type CommonWireguardFields, TauriEventKey } from '../types';

type Props = PropsWithChildren;

export const MfaModalProvider = ({ children }: Props) => {
  const openMFAModal = useMFAModal((state) => state.open);
  // listen for Rust backend requesting MFA

  useEffect(() => {
    let unlisten: UnlistenFn;

    (async () => {
      unlisten = await listen<CommonWireguardFields>(
        TauriEventKey.MFA_TRIGGER,
        ({ payload: location }) => {
          if (isPresent(location)) {
            location.connection_type = ClientConnectionType.LOCATION;
            openMFAModal(location);
          }
        },
      );
    })();

    return () => {
      unlisten?.();
    };
  }, [openMFAModal]);

  return (
    <>
      {children}
      <MFAModal />
    </>
  );
};
