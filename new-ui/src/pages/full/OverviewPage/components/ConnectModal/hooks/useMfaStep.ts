import { useShallow } from 'zustand/shallow';
import { isPresent } from '../../../../../../shared/utils/isPresent';
import { usableMfaMethods } from '../../../../../../shared/utils/mfa';
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

  const currentStep = location?.mfa_steps[stepIndex];

  return {
    canPickOtherMethod:
      isPresent(currentStep) && usableMfaMethods(currentStep).length > 1,
    stepPlan,
    mfaToken,
    setMfaToken,
    goToStep,
  };
};
