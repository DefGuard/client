import './style.scss';
import { useMemo, useState } from 'react';
import { MfaSelector } from '../../../../shared/components/LocationCard/components/MfaSelector/MfaSelector';
import { MfaMethod, type MfaMethodValue } from '../../../../shared/rust-api/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const steps: MfaMethodValue[][] = [
  [MfaMethod.Totp, MfaMethod.Email, MfaMethod.MobileApprove],
  [MfaMethod.Oidc, MfaMethod.Biometric],
  [MfaMethod.Totp, MfaMethod.Email],
];

export const PlaygroundConfigureMfa = () => {
  const [selected, setSelected] = useState<(MfaMethodValue | null)[]>(
    steps.map(() => null),
  );

  const result = useMemo(
    (): MfaMethodValue[] => selected.filter((factor) => factor !== null),
    [selected],
  );

  const selectFactor = (stepIndex: number, factor: MfaMethodValue) => {
    setSelected((prev) =>
      prev.map((current, index) => (index === stepIndex ? factor : current)),
    );
  };

  return (
    <PlaygroundCard>
      <div id="configure-mfa-playground">
        <div className="debug-header">
          <p>Result: {result.length === steps.length ? JSON.stringify(result) : '—'}</p>
        </div>
        <div className="steps">
          {steps.map((factors, stepIndex) => (
            <div className="step" key={stepIndex}>
              <p>Step {stepIndex + 1}</p>
              {factors.map((factor) => (
                <MfaSelector
                  key={factor}
                  factor={factor}
                  selected={selected[stepIndex] === factor}
                  onClick={() => selectFactor(stepIndex, factor)}
                />
              ))}
            </div>
          ))}
        </div>
      </div>
    </PlaygroundCard>
  );
};
