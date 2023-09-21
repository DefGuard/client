import { ReactNode, useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useAddInstanceModal } from './hooks/useAddInstanceModal';
import { AddInstanceDeviceStep } from './steps/AddInstanceDeviceStep';
import { AddInstanceModalInitStep } from './steps/AddInstanceInitStep';

export const AddInstanceModal = () => {
  const { LL } = useI18nContext();
  const componentLL = LL.pages.client.modals.addInstanceModal;
  const [isOpen, loading, currentStep] = useAddInstanceModal((state) => [
    state.isOpen,
    state.loading,
    state.step,
  ]);

  const [reset, close] = useAddInstanceModal(
    (state) => [state.reset, state.close],
    shallow,
  );

  const getTitle = useMemo(() => {
    switch (currentStep) {
      case 0:
        return componentLL.titles.addInstance();
      case 1:
        return componentLL.titles.addDevice();
      default:
        return '';
    }
  }, [currentStep, componentLL]);

  return (
    <ModalWithTitle
      title={getTitle}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      disableClose={loading || currentStep > 0}
      steps={steps}
      currentStep={currentStep}
      backdrop
    />
  );
};

const steps: ReactNode[] = [
  <AddInstanceModalInitStep key={0} />,
  <AddInstanceDeviceStep key={1} />,
];
