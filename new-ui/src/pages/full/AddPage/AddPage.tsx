import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import {
  getInstancesQueryOptions,
  mfaConfigurableInstances,
  tunnelsDisabled,
} from '../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../shared/types';
import { AddCard } from './components/AddCard/AddCard';
import { useStartMfaConfiguration } from './hooks/useStartMfaConfiguration';

export const AddPage = () => {
  const navigate = useNavigate();
  const { data: instances, isPending } = useQuery(getInstancesQueryOptions);

  const mfaInstances = useMemo(
    () => mfaConfigurableInstances(instances ?? []),
    [instances],
  );

  const { mutate: startConfigureMfa, isPending: configureMfaPending } =
    useStartMfaConfiguration();

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
        {!isPending && !tunnelsDisabled(instances ?? []) && (
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
        {!isPending && mfaInstances.length > 0 && (
          <AddCard
            image="lock"
            title="Add new MFA method"
            description={`Add new MFA methods to securely authenticate when connecting to a location, with flexible options to match your security requirements.`}
            actionText="Add MFA"
            onClick={() => {
              // Below two instances there is nothing to pick, so the flow starts here.
              if (mfaInstances.length > 1) {
                navigate({
                  to: '/full/add/mfa',
                });
                return;
              }
              startConfigureMfa(mfaInstances[0]);
            }}
            loading={configureMfaPending}
          />
        )}
      </div>
    </FullPage>
  );
};
