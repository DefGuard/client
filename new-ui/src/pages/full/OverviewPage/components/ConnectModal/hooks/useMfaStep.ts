import { useShallow } from 'zustand/shallow';
import { useAppData } from '../../../../../../shared/providers/AppDataContext';
import { isPresent } from '../../../../../../shared/utils/isPresent';
import { mfaStepsOf, usableMfaMethods } from '../../../../../../shared/utils/mfa';
import { useConnectModal } from './useConnectModal';

export const useMfaStep = () => {
  const [location, stepIndex, stepPlan, mfaToken, setMfaToken, goToStep] =
    useConnectModal(
      useShallow((s) => [
        s.location,
        s.stepIndex,
        s.stepPlan,
        s.mfaToken,
        s.setMfaToken,
        s.goToStep,
      ]),
    );

  const { instances } = useAppData();
  const instance = instances.find((entry) => entry.id === location?.instance_id);

  const currentStep = isPresent(location) ? mfaStepsOf(location)[stepIndex] : undefined;

  return {
    canPickOtherMethod:
      isPresent(currentStep) && usableMfaMethods(currentStep, instance).length > 1,
    stepPlan,
    mfaToken,
    setMfaToken,
    goToStep,
  };
};
