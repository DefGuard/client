import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { useAppData } from '../../../../../shared/providers/AppDataContext';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import bannerSrc from './assets/banner.png';

export const ConfigureFinishStep = () => {
  const navigate = useNavigate();
  const { setViewSelection } = useAppData();
  const instance = useConfigureMfaStore((s) => s.instance);

  return (
    <div id="configure-finish-step" className="step-content">
      <img className="banner" src={bannerSrc} loading="eager" width={504} height={150} />
      <header>
        <h1>MFA method(s) have been successfully added.</h1>
      </header>
      <p className="summary">{`You can now use them to access MFA protected locations.`}</p>
      <Controls>
        <div className="right">
          <Button
            text="Finish"
            variant={ButtonVariant.Primary}
            onClick={() => {
              if (isPresent(instance)) {
                setViewSelection({ kind: 'instance', id: instance.id });
              }
              // Reset once the page is gone, or this step re-renders on an empty store.
              void navigate({ to: '/full/overview', replace: true }).then(() => {
                void discardMfaConfiguration();
              });
            }}
          />
        </div>
      </Controls>
    </div>
  );
};
