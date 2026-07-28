import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { getInstancesQueryOptions } from '../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../shared/types';
import { AddCard } from './components/AddCard/AddCard';

export const AddPage = () => {
  const navigate = useNavigate();
  const { data: instances } = useQuery(getInstancesQueryOptions);
  const tunnelsDisabled = instances?.some((i) => i.disable_tunnels) ?? false;
  return (
    <FullPage id="add-page-view">
      <FullPageTitle title="Add Defguard items" spacing={ThemeSpacing.Xl} />
      <div className="cards">
        <AddCard
          image="default"
          onClick={() => {
            navigate({
              to: '/full/add/instance',
            });
          }}
          title="Add Instance"
          actionText="Add instance"
          description={`Establish a secure connection to your Defguard instance effortlessly by configuring it with a single token—no manual setup.`}
        />
        {!tunnelsDisabled && (
          <AddCard
            image="wireguard"
            onClick={() => {
              navigate({
                to: '/full/add/tunnel',
              });
            }}
            title="Add WireGuard Tunnel"
            actionText="Add tunnel"
            description={`Add and configure a WireGuard tunnel to securely route traffic through an encrypted connection using predefined configuration.`}
          />
        )}
      </div>
    </FullPage>
  );
};
