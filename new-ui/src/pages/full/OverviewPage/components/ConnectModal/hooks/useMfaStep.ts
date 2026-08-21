import { useShallow } from 'zustand/shallow';
import { isPresent } from '../../../../../../shared/utils/isPresent';
import { mfaStepCount, usableMfaMethods } from '../../../../../../shared/utils/mfa';
import { useConnectModal } from './useConnectModal';

export const useMfaStep = () => {
  const [location, stepIndex, passStep] = useConnectModal(
    useShallow((s) => [s.location, s.stepIndex, s.passStep]),
  );

  const currentStep = location?.mfa_steps[stepIndex];
  const isMultiStep = isPresent(location) && mfaStepCount(location) > 1;

  return {
    canPickOtherMethod:
      isPresent(currentStep) && usableMfaMethods(currentStep).length > 1,
    onStepPassed: isMultiStep ? passStep : undefined,
  };
};
