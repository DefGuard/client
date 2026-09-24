import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import clsx from 'clsx';
import { useCallback, useEffect, useMemo } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { LoaderSpinner } from '../../../shared/components/LoaderSpinner/LoaderSpinner';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import {
  getInstancesQueryOptions,
  mfaConfigurableInstances,
} from '../../../shared/rust-api/query';
import { isPresent } from '../../../shared/utils/isPresent';
import { useStartMfaConfiguration } from '../AddPage/hooks/useStartMfaConfiguration';

/** Shown when the client is enrolled into more than one instance, to pick which to configure. */
export const SelectMfaInstancePage = () => {
  const navigate = useNavigate();
  const { data: instances } = useQuery(getInstancesQueryOptions);

  const mfaInstances = useMemo(
    () => mfaConfigurableInstances(instances ?? []),
    [instances],
  );

  const {
    mutate: startConfigureMfa,
    isPending,
    variables: opening,
  } = useStartMfaConfiguration();

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
      <FullPageTitle title="Choose an instance" />
      <p className="description">
        <span>{`Pick the Defguard instance to add a multi-factor authentication method to.`}</span>
      </p>
      <div className="instances">
        {mfaInstances.map((instance) => {
          const pending = isPending && opening?.id === instance.id;
          return (
            <div className="instance-row" key={instance.id}>
              <button
                className={clsx('instance', { pending })}
                // One session at a time, the rest wait until this one resolves.
                disabled={isPending}
                type="button"
                onClick={() => {
                  startConfigureMfa(instance);
                }}
              >
                <span>{instance.name}</span>
                {pending && <LoaderSpinner size={18} variant="primary" />}
              </button>
            </div>
          );
        })}
      </div>
      <Controls>
        <Button text="Cancel" variant={ButtonVariant.Secondary} onClick={leave} />
      </Controls>
    </FullPage>
  );
};
