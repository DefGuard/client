import { useMutation } from '@tanstack/react-query';
import { Fragment, useCallback, useMemo, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { FieldError } from '../../../../../shared/components/FieldError/FieldError';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import {
  MfaMethod,
  type MfaMethodValue,
  type MfaStep,
} from '../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  isDesktopDrivableMethod,
  mfaStepsOf as locationMfaSteps,
  mfaToText,
} from '../../../../../shared/utils/mfa';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { isMfaFactorOfferable, MFA_CONFIGURABLE_FACTORS } from '../../types';
import '../style.scss';
import './style.scss';
import { MethodRow, type MethodRowState } from './components/MethodRow';

interface Props {
  onCancel: () => void;
}

const CLIENT_CONFIGURABLE_METHODS = MFA_CONFIGURABLE_FACTORS.map(
  (factor) => factor.method,
);

export const ConfigureSelectMethodsStep = ({ onCancel }: Props) => {
  const configuredMethods = useConfigureMfaStore((s) => s.configuredMethods);
  const emailFallback = useConfigureMfaStore((s) => s.emailFallback);
  const location = useConfigureMfaStore((s) => s.location);

  const [selected, setSelected] = useState<MfaMethodValue[]>([]);
  const [error, setError] = useState<string | null>(null);

  const locationSteps = useMemo(
    () => (isPresent(location) ? locationMfaSteps(location) : []),
    [location],
  );
  const isLocationAware = locationSteps.length > 0;

  /** The fallback registers email as it mails the code, so email is disabled but not
   *  something the account already holds. */
  const accountConfigured = useMemo(() => {
    if (!emailFallback) return configuredMethods;
    return configuredMethods.filter((method) => method !== MfaMethod.Email);
  }, [emailFallback, configuredMethods]);

  /** A step the user is already past: it offers a factor the desktop can drive and the account
   *  holds. Mirrors `usableMfaMethods`, on the session's fresher list of configured factors. */
  const isStepSatisfied = useCallback(
    (step: MfaStep): boolean =>
      step.methods.some(
        (entry) =>
          isDesktopDrivableMethod(entry.method) &&
          configuredMethods.includes(entry.method),
      ),
    [configuredMethods],
  );

  const groups = useMemo(() => {
    let result = [CLIENT_CONFIGURABLE_METHODS];
    if (isLocationAware) {
      result = locationSteps.map((step) => step.methods.map((entry) => entry.method));
    }
    // With a single list there is no step order to preserve, so park what can't be
    // picked at the bottom. Sort is stable, keeping the listing order within a bucket.
    if (result.length === 1) {
      return [
        [...result[0]].sort(
          (a, b) =>
            Number(!isMfaFactorOfferable(a, configuredMethods)) -
            Number(!isMfaFactorOfferable(b, configuredMethods)),
        ),
      ];
    }
    return result;
  }, [isLocationAware, locationSteps, configuredMethods]);

  const describeMethod = useCallback(
    (method: MfaMethodValue): MethodRowState => {
      // A repeatable factor stays offerable once configured, keeping its badge and its pick.
      const disabled = !isMfaFactorOfferable(method, configuredMethods);
      const configured = accountConfigured.includes(method);
      const satisfied = configuredMethods.includes(method);

      let hint: string | null = null;
      if (disabled && !configured) {
        if (satisfied) {
          // Reached only via the email fallback, which registers the factor as it verifies.
          hint = `${mfaToText(method)} is required to continue.`;
        } else {
          hint = `${mfaToText(method)} cannot be configured in the desktop client.`;
        }
      }

      return {
        method,
        // Pre-ticked only for a factor that answers a step on its own; a mobile-only one does
        // not, however configured it is.
        checked:
          selected.includes(method) ||
          (disabled && satisfied && isDesktopDrivableMethod(method)),
        disabled,
        configured,
        hint,
      };
    },
    [accountConfigured, configuredMethods, selected],
  );

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

  const toggle = useCallback((method: MfaMethodValue) => {
    setError(null);
    // Kept in listing order, which is the order the wizard sets them up in.
    setSelected((current) => {
      if (current.includes(method)) {
        return current.filter((picked) => picked !== method);
      }
      return CLIENT_CONFIGURABLE_METHODS.filter(
        (candidate) => candidate === method || current.includes(candidate),
      );
    });
  }, []);

  const handleSubmit = useCallback(() => {
    if (isLocationAware) {
      const unanswered = locationSteps.findIndex(
        (step) =>
          !isStepSatisfied(step) &&
          !step.methods.some((entry) => selected.includes(entry.method)),
      );
      if (unanswered !== -1) {
        setError(
          locationSteps.length > 1
            ? `Select a method for step ${unanswered + 1}`
            : 'Select at least one method',
        );
        return;
      }
      // Every step reads as answered yet there is nothing to set up, so continuing would drop
      // the user on the closing screen having configured nothing.
      if (selected.length === 0) {
        setError('Select at least one method');
        return;
      }
    } else if (selected.length === 0 && !emailFallback) {
      // The fallback configures email on its own, so it satisfies the one-factor minimum.
      setError('Select at least one method');
      return;
    }
    useConfigureMfaStore.getState().selectMethods(selected);
  }, [selected, emailFallback, isLocationAware, locationSteps, isStepSatisfied]);

  const renderMethod = (method: MfaMethodValue) => (
    <MethodRow key={method} {...describeMethod(method)} onToggle={toggle} />
  );

  return (
    <FullPage
      id="configure-select-methods-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle
        title={isLocationAware ? 'Configure MFA' : 'Choose the methods to configure'}
      />
      <p className="description">
        {isLocationAware && isPresent(location) ? (
          <>
            <span>{`For additional security, your company has introduced new access requirements for ${location.name}. The setup is simple and should only take a few minutes.`}</span>
            <span>{`If multiple authentication options are available, you can choose your preferred method below. Otherwise, simply start the setup process to get access to the location.`}</span>
          </>
        ) : (
          <>
            <span>{`Pick the multi-factor authentication methods you want to set up.`}</span>
            <span>{`Methods already configured on your account are listed for reference.`}</span>
          </>
        )}
      </p>
      <div className="steps">
        {groups.map((methods, index) => (
          // Index as key: Core owns the order and it never reorders here.
          <Fragment key={index}>
            {index > 0 && <Divider spacing={ThemeSpacing.Lg} />}
            <div className="step">
              {groups.length > 1 && <p className="step-label">{`Step ${index + 1}:`}</p>}
              <div className="methods">{methods.map(renderMethod)}</div>
            </div>
          </Fragment>
        ))}
      </div>
      <FieldError error={error} />
      <Controls>
        <Button
          text="Cancel"
          variant={ButtonVariant.Secondary}
          loading={isCancelling}
          onClick={() => {
            cancel();
          }}
        />
        <div className="right">
          <Button
            text="Continue"
            variant={ButtonVariant.Primary}
            // Cancel already discarded the session, nothing left to continue into.
            disabled={isCancelling}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
