import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import bannerSrc from './assets/banner.png';

export const FinishStep = () => {
  const navigate = useNavigate();
  return (
    <div id="finish-step" className="step-content">
      <div className="content-pad">
        <img src={bannerSrc} loading="eager" width={504} height={150} />
        <div className="top">
          <p>{`New Defguard instance added successfully`}</p>
          <p>{`You can now connect this device, check its status and view statistics.`}</p>
        </div>
        <div className="markdown-content"></div>
      </div>
      <Controls>
        <div className="right">
          <Button
            variant={ButtonVariant.Primary}
            text="Got it"
            onClick={() => {
              navigate({ to: '/full/overview', replace: true });
            }}
          />
        </div>
      </Controls>
    </div>
  );
};
