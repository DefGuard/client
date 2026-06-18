import './styles.scss';
import { useNavigate } from '@tanstack/react-router';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { useTunnelWizardStore } from '../TunnelWizardPage/hooks/useTunnelWizardStore';

export const AddTunnelPage = () => {
  const navigate = useNavigate();
  return (
    <FullPage id="add-tunnel-page">
      <FullPageTitle title="Add WireGuard Tunnel" />
      <div className="contents">
        <p className="page-description">{`A WireGuard tunnel is a secure, encrypted connection that allows your device or network to communicate safely over the internet.It ensures that your data is protected and transmitted through a private, trusted channel.`}</p>
        <p className="page-description">{`To create a WireGuard tunnel, you'll need to provide a set of connection details. You can do this automatically by uploading a configuration file or set everything up manually.`}</p>
      </div>
      <Controls>
        <Button
          variant={ButtonVariant.Secondary}
          text="Back"
          onClick={() => {
            navigate({ to: '/full/add' });
          }}
        />
        <div className="right">
          <Button
            variant={ButtonVariant.Primary}
            text="Add tunnel"
            onClick={() => {
              useTunnelWizardStore.getState().reset();
              navigate({ to: '/full/tunnel-wizard' });
            }}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
