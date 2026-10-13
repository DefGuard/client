import '../ConfigureMfaPage/verify/style.scss';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { fork } from 'radashi';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { Icon, IconKind } from '../../../shared/components/Icon';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { TooltipContent } from '../../../shared/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../shared/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../shared/providers/tooltip/TooltipTrigger';
import { getInstancesQueryOptions } from '../../../shared/rust-api/query';
import { ThemeSpacing, ThemeVariable } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { supportsMfaConfiguration } from '../../../shared/utils/mfa';
import { InstanceSelector } from '../ConfigureMfaPage/components/InstanceSelector/InstanceSelector';
import { useStartMfaConfiguration } from './hooks/useStartMfaConfiguration';

export const SelectMfaInstancePage = () => {
  const navigate = useNavigate();
  const { data: instances } = useQuery(getInstancesQueryOptions);
  const [selectedId, setSelectedId] = useState<number>();

  const [available, incompatible] = useMemo(
    () => fork(instances ?? [], supportsMfaConfiguration),
    [instances],
  );

  // Derived so a poll dropping the picked instance also clears the selection.
  const selectedInstance = useMemo(() => {
    const id = selectedId ?? (available.length === 1 ? available[0].id : undefined);
    return available.find((instance) => instance.id === id);
  }, [available, selectedId]);

  const { mutate: startConfigureMfa, isPending } = useStartMfaConfiguration();

  const leave = useCallback(() => {
    navigate({ to: '/full/add' });
  }, [navigate]);

  // The route guard only runs on entry, so a poll leaving nothing to pick sends the user back.
  useEffect(() => {
    if (isPresent(instances) && available.length === 0) {
      leave();
    }
  }, [instances, available.length, leave]);

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
      <div className="label">
        <p>Available instances</p>
      </div>
      <SizedBox height={ThemeSpacing.Md} />
      <div className="instances">
        {available.map((instance) => (
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
      {incompatible.length > 0 && (
        <>
          <SizedBox height={ThemeSpacing.Xl4} />
          <div className="label">
            <p>Incompatible instances</p>
            <TooltipProvider>
              <TooltipTrigger>
                <Icon
                  icon={IconKind.Question}
                  size={16}
                  staticColor={ThemeVariable.FgWhite60}
                />
              </TooltipTrigger>
              <TooltipContent>
                <p>{`Some instances may be running an older version and may not be compatible with the MFA configuration feature.`}</p>
              </TooltipContent>
            </TooltipProvider>
          </div>
          <SizedBox height={ThemeSpacing.Md} />
          <div className="instances">
            {incompatible.map((instance) => (
              <InstanceSelector
                key={instance.id}
                instanceId={instance.id}
                instanceName={instance.name}
                selected={false}
                disabled
              />
            ))}
          </div>
        </>
      )}
      <SizedBox height={ThemeSpacing.Xl} />
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
