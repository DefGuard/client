import './style.scss';

import { useNavigate } from '@tanstack/react-router';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { useTunnelWizardStore } from '../../hooks/useTunnelWizardStore';
import bannerSrc from './assets/banner.png';

export const FinishStep = () => {
  const navigate = useNavigate();

  return (
    <div id="finish-step">
      <div className="banner">
        <img src={bannerSrc} height={150} width={536} />
      </div>
      <h1>Your WireGuard tunnel added successfully</h1>
      <p>You can now connect this device, check its status and view statistics.</p>
      <div className="actions">
        <Button
          text="Add another tunnel"
          onClick={() => {
            useTunnelWizardStore.getState().reset();
          }}
          variant={ButtonVariant.Secondary}
        />
        <Button
          text="Got it"
          onClick={() => {
            navigate({ to: '/full/overview', replace: true });
            setTimeout(() => {
              useTunnelWizardStore.getState().reset();
            }, 250);
          }}
          variant={ButtonVariant.Primary}
        />
      </div>
    </div>
  );
};
