import { useState } from 'react';

import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { AddInstanceDeviceForm } from './components/AddInstanceDeviceForm/AddInstanceDeviceForm';
import { AddInstanceInitForm } from './components/AddInstanceInitForm/AddInstanceInitForm';
import { AddInstnaceInitResponse } from './types';

export const AddInstanceFormCard = () => {
  const [currentStep, setCurrentStep] = useState(0);
  const [response, setResponse] = useState<AddInstnaceInitResponse | undefined>(
    undefined,
  );
  return (
    <Card id="add-instance-form-card">
      {currentStep === 0 && (
        <AddInstanceInitForm
          nextStep={(data) => {
            setResponse(data);
            setCurrentStep((step) => step + 1);
          }}
        />
      )}
      {currentStep === 1 && response && <AddInstanceDeviceForm response={response} />}
    </Card>
  );
};
