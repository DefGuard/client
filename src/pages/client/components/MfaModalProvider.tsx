import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { type PropsWithChildren, useEffect } from 'react';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { MFAModal } from '../pages/ClientInstancePage/components/LocationsList/modals/MFAModal/MFAModal';
import { useMFAModal } from '../pages/ClientInstancePage/components/LocationsList/modals/MFAModal/useMFAModal';
import type { CommonWireguardFields } from '../types';

type Props = PropsWithChildren;

type Payload = {
  location?: CommonWireguardFields;
};

export const MfaModalProvider = ({ children }: Props) => {
  const openMFAModal = useMFAModal((state) => state.open);
  // listen for rust backend requesting MFA

  useEffect(() => {
    let unlisten: UnlistenFn;

    (async () => {
      unlisten = await listen<Payload>('mfa-trigger', ({ payload: { location } }) => {
        if (isPresent(location)) {
          openMFAModal(location);
        }
      });
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
