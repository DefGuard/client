/** biome-ignore-all lint/style/noNonNullAssertion: query ensured by enabled flag */
import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { Fragment, type ReactNode, useEffect } from 'react';
import { useShallow } from 'zustand/shallow';
import { Modal } from '../../../../../shared/components/Modal/Modal';
import { useAppData } from '../../../../../shared/providers/AppDataContext';
import { api } from '../../../../../shared/rust-api/api';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { mfaStepCount, mfaToText } from '../../../../../shared/utils/mfa';
import {
  ConnectModalTitle,
  ConnectModalView,
  type ConnectModalViewValue,
  mfaMethodToConnectModalView,
} from './hooks/types';
import { useConnectModal } from './hooks/useConnectModal';
import { ConnectModalConnectionError } from './views/ConnectModalConnectionError/ConnectModalConnectionError';
import { ConnectModalMfaEmail } from './views/ConnectModalMfaEmail/ConnectModalMfaEmail';
import { ConnectModalMfaFido2 } from './views/ConnectModalMfaFido2/ConnectModalMfaFido2';
import { ConnectModalMfaMobile } from './views/ConnectModalMfaMobile/ConnectModalMfaMobile';
import { ConnectModalMfaOidc } from './views/ConnectModalMfaOidc/ConnectModalMfaOidc';
import { ConnectModalMfaSettings } from './views/ConnectModalMfaSettings/ConnectModalMfaSettings';
import { ConnectModalMfaTotp } from './views/ConnectModalMfaTotp/ConnectModalMfaTotp';
import { ConnectModalPostureCheckFail } from './views/ConnectModalPostureCheckFail/ConnectModalPostureCheckFail';

export const ConnectModal = () => {
  const reset = useConnectModal((s) => s.reset);

  const [view, visible, location, stepIndex, stepPlan] = useConnectModal(
    useShallow((s) => [s.view, s.visible, s.location, s.stepIndex, s.stepPlan]),
  );

  const stepCount = isPresent(location) ? mfaStepCount(location) : 0;
  const stepMethod = stepPlan[stepIndex];
  const isOnMfaStepView =
    isPresent(stepMethod) && view === mfaMethodToConnectModalView(stepMethod);
  const stepLabel =
    stepCount > 1 && isOnMfaStepView
      ? `Step ${stepIndex + 1}/${stepCount}: ${mfaToText(stepMethod)}`
      : null;

  const isOpen = isPresent(view) && isPresent(location) && visible;

  return (
    <Modal
      id="connect-modal"
      size="small"
      title={stepLabel ?? (view ? ConnectModalTitle[view] : '')}
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
  [ConnectModalView.MfaFido2]: <ConnectModalMfaFido2 />,
  [ConnectModalView.MfaSettings]: <ConnectModalMfaSettings />,
  [ConnectModalView.PostureCheckFail]: <ConnectModalPostureCheckFail />,
  [ConnectModalView.ConnectionError]: <ConnectModalConnectionError />,
} as const;

const ModalContent = () => {
  const { setConnectionMethod } = useAppData();
  const storeLocation = useConnectModal((s) => s.location);
  const { data: activeConnection, isFetching } = useQuery({
    queryKey: ['active-connection', storeLocation?.id, storeLocation?.connection_type],
    queryFn: () =>
      api.getActiveConnection({
        locationId: storeLocation!.id,
        connectionType: storeLocation!.connection_type,
      }),
    enabled: isPresent(storeLocation),
    retry: false,
  });
  const activeView = useConnectModal((s) => s.view);
  const stepIndex = useConnectModal((s) => s.stepIndex);

  // When user completes connection and it's working modal is no longer needed so auto close it
  // biome-ignore lint/correctness/useExhaustiveDependencies: side-effect on connect
  useEffect(() => {
    if (!isFetching && isPresent(activeConnection) && isPresent(storeLocation)) {
      const mfaMethod = useConnectModal.getState().mfaMethod;
      setConnectionMethod(storeLocation.id, storeLocation.connection_type, mfaMethod);
      useConnectModal.setState({ visible: false });
    }
  }, [activeConnection, isFetching]);

  if (!activeView) return null;

  return (
    <Fragment key={`${activeView}-${stepIndex}`}>{viewContent[activeView]}</Fragment>
  );
};
