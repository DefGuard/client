import '../ConfigureMfaPage/verify/style.scss';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { getInstancesQueryOptions } from '../../../shared/rust-api/query';
import { isPresent } from '../../../shared/utils/isPresent';
import { mfaConfigurableInstances } from '../../../shared/utils/mfa';
import { useStartMfaConfiguration } from '../AddPage/hooks/useStartMfaConfiguration';
import { InstanceSelector } from '../ConfigureMfaPage/components/InstanceSelector/InstanceSelector';

/** Shown when the client is enrolled into more than one instance, to pick which to configure. */
export const SelectMfaInstancePage = () => {
  const navigate = useNavigate();
  const { data: instances } = useQuery(getInstancesQueryOptions);
  const [selectedId, setSelectedId] = useState<number>();

  const mfaInstances = useMemo(
    () => mfaConfigurableInstances(instances ?? []),
    [instances],
  );

  // Derived so a poll dropping the picked instance also clears the selection.
  const selectedInstance = useMemo(
    () => mfaInstances.find((instance) => instance.id === selectedId),
    [mfaInstances, selectedId],
  );

  const { mutate: startConfigureMfa, isPending } = useStartMfaConfiguration();

  const leave = useCallback(() => {
    navigate({ to: '/full/add' });
  }, [navigate]);

  // The route guard only runs on entry, so a poll leaving nothing to pick sends the user back.
  useEffect(() => {
    if (isPresent(instances) && mfaInstances.length < 2) {
      leave();
    }
  }, [instances, mfaInstances.length, leave]);

  return (
    <FullPage
      id="select-mfa-instance-page"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Select instance" />
      <p className="description">
        <span>{`To configure a new MFA method, first select the instance you want to configure it for.`}</span>
      </p>
      <p className="label">Your instances</p>
      <div className="instances">
        {mfaInstances.map((instance) => (
          <InstanceSelector
            key={instance.id}
            instanceId={instance.id}
            instanceName={instance.name}
            selected={selectedInstance?.id === instance.id}
            onClick={() => {
              // Keep the pick fixed while its session is opening.
              if (!isPending) {
                setSelectedId(instance.id);
              }
            }}
          />
        ))}
      </div>
      <Controls>
        <Button text="Cancel" variant={ButtonVariant.Secondary} onClick={leave} />
        <div className="right">
          <Button
            text="Continue"
            variant={ButtonVariant.Primary}
            disabled={!isPresent(selectedInstance)}
            loading={isPending}
            onClick={() => {
              if (!isPresent(selectedInstance)) return;
              startConfigureMfa(selectedInstance);
            }}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
