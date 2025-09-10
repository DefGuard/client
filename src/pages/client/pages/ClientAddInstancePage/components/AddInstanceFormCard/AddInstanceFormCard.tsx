import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { AddInstanceFormStep } from '../../hooks/types';
import { useAddInstanceStore } from '../../hooks/useAddInstanceStore';
import { AddInstanceDeviceForm } from './components/AddInstanceDeviceForm/AddInstanceDeviceForm';
import { AddInstanceInitForm } from './components/AddInstanceInitForm/AddInstanceInitForm';

export const AddInstanceFormCard = () => {
  const currentStep = useAddInstanceStore((s) => s.step);
  return (
    <Card id="add-instance-form-card">
      {currentStep === AddInstanceFormStep.INIT && <AddInstanceInitForm />}
      {currentStep === AddInstanceFormStep.DEVICE && <AddInstanceDeviceForm />}
    </Card>
  );
};
