import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useAppData } from '../../../providers/AppDataContext';
import { api } from '../../../rust-api/api';
import type { InstanceInfo, LocationInfo, MfaStep } from '../../../rust-api/types';
import { ConnectionType, MfaMethod, type MfaMethodValue } from '../../../rust-api/types';
import { useAppStore } from '../../../store/useAppStore';
import { isPresent } from '../../../utils/isPresent';
import { isMfaMethodUsable, mfaToText, shouldStartMfa } from '../../../utils/mfa';
import {
  LocationCardViews,
  type LocationCardViewsValue,
  mfaMethodToLocationCardView,
} from './types';

interface LocationCardContextValue {
  location: LocationInfo;
  instance?: InstanceInfo;
  currentView: LocationCardViewsValue;
  previousView: LocationCardViewsValue | null;
  postureError: string | null;
  autoConnectOpenid: boolean;
  mfaMethod: MfaMethodValue;
  canPickOtherMethod: boolean;
  stepPlan: MfaMethodValue[];
  stepIndex: number;
  stepLabel: string | null;
  onStepPassed?: () => void;
  setMfaMethod: (value: MfaMethodValue) => void;
  setStepPlanOnce: (plan: MfaMethodValue[]) => void;
  setView: (view: LocationCardViewsValue) => void;
  setPostureError: (error: string | null) => void;
  startMfa: () => void;
}

const LocationCardContext = createContext<LocationCardContextValue | null>(null);

export const useLocationCardContext = (): LocationCardContextValue => {
  const ctx = useContext(LocationCardContext);
  if (!ctx) {
    throw new Error('useLocationCardContext must be used within a LocationCardProvider');
  }
  return ctx;
};

interface LocationCardProviderProps {
  instance?: InstanceInfo;
  location: LocationInfo;
  children: ReactNode;
}

export const LocationCardProvider = ({
  location,
  instance,
  children,
}: LocationCardProviderProps) => {
  const conTypeSetOnce = useRef(false);
  const mfaStarted = useRef(false);
  const { connectionMfaMethod, setConnectionMethod } = useAppData();
  const [autoConnectOpenid, setAutoConnectOpenid] = useState(false);
  const [previousView, setPreviousView] = useState<LocationCardViewsValue | null>(null);
  const [postureError, setPostureError] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<LocationCardViewsValue>(
    location.active ? LocationCardViews.Connected : LocationCardViews.Default,
  );
  const [mfaMethod, setMfaMethod] = useState<MfaMethodValue>(
    location.mfa_method ?? MfaMethod.Totp,
  );

  const mfaSteps = useMemo<MfaStep[]>(
    () => (shouldStartMfa(location) ? location.mfa_steps : []),
    [location],
  );
  const isMultiStep = mfaSteps.length > 1;

  // one-off choice made through "Other methods", dropped when a new flow starts
  const [stepPlanOnce, setStepPlanOnce] = useState<MfaMethodValue[]>([]);
  const stepPlan = useMemo<MfaMethodValue[]>(
    () =>
      mfaSteps.map((step, index) => {
        const usable = step.methods.filter(isMfaMethodUsable);
        const chosen = [stepPlanOnce[index], location.mfa_step_plan[index]].find(
          (method) => usable.some((entry) => entry.method === method),
        );
        return chosen ?? (usable[0] ?? step.methods[0]).method;
      }),
    [mfaSteps, stepPlanOnce, location.mfa_step_plan],
  );
  const [stepIndex, setStepIndex] = useState(0);

  // Other location updates must not undo an optimistic connection transition.
  // biome-ignore lint/correctness/useExhaustiveDependencies: synchronize only on active state
  useEffect(() => {
    if (location.active) {
      setCurrentView(LocationCardViews.Connected);
    } else {
      setMfaMethod(location.mfa_method ?? MfaMethod.Totp);
      setCurrentView(LocationCardViews.Default);
      setStepIndex(0);
    }
  }, [location.active]);

  const setView = useCallback(
    (view: LocationCardViewsValue) => {
      setPreviousView(currentView);
      setCurrentView(view);
    },
    [currentView],
  );

  const passStep = useCallback(() => {
    const next = stepIndex + 1;
    if (next < stepPlan.length) {
      setStepIndex(next);
      setView(mfaMethodToLocationCardView(stepPlan[next]));
      return;
    }
    setStepIndex(0);
    // TODO(mock): the last step ends the flow without connecting; connect here once
    // MfaCompleted brings up the tunnel
    setView(LocationCardViews.Default);
  }, [setView, stepPlan, stepIndex]);

  const onStepPassed = isMultiStep ? passStep : undefined;

  const startMfa = useCallback(async () => {
    mfaStarted.current = true;
    const appConfig = await api.getAppConfig();
    setAutoConnectOpenid(appConfig.auto_start_openid_mfa);
    if (isMultiStep) {
      setStepPlanOnce([]);
      setStepIndex(0);
      setView(mfaMethodToLocationCardView(stepPlan[0]));
      return;
    }
    setView(mfaMethodToLocationCardView(mfaMethod));
  }, [setView, mfaMethod, isMultiStep, stepPlan]);

  const mfaAutoStartRequested = useAppStore(
    (s) => s.mfaAutoStartLocationId === location.id,
  );
  useEffect(() => {
    if (
      mfaAutoStartRequested &&
      location.connection_type === ConnectionType.Location &&
      !location.active
    ) {
      useAppStore.setState({ mfaAutoStartLocationId: null });
      void startMfa();
    }
  }, [mfaAutoStartRequested, location.connection_type, location.active, startMfa]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side-effect on location.active
  useEffect(() => {
    if (
      location.active &&
      location.connection_type !== ConnectionType.Tunnel &&
      !conTypeSetOnce.current
    ) {
      const key = `${location.connection_type.toLowerCase()}-${location.id}`;
      if (mfaStarted.current || !isPresent(connectionMfaMethod[key])) {
        conTypeSetOnce.current = true;
        setConnectionMethod(location.id, location.connection_type, mfaMethod);
      }
    }
    if (!location.active) {
      conTypeSetOnce.current = false;
      mfaStarted.current = false;
    }
  }, [location.active]);

  const usableStepMethods = (mfaSteps[stepIndex]?.methods ?? []).filter(
    isMfaMethodUsable,
  );
  const canPickOtherMethod = !isMultiStep || usableStepMethods.length > 1;

  const stepMethod = stepPlan[stepIndex];
  const showStepLabel = isMultiStep && isPresent(stepMethod);
  const stepLabel = showStepLabel
    ? `Step ${stepIndex + 1}/${mfaSteps.length}: ${mfaToText(stepMethod)}`
    : null;

  return (
    <LocationCardContext.Provider
      value={{
        currentView,
        previousView,
        postureError,
        autoConnectOpenid,
        location,
        instance,
        mfaMethod,
        canPickOtherMethod,
        stepPlan,
        stepIndex,
        stepLabel,
        onStepPassed,
        setView,
        setPostureError,
        startMfa,
        setMfaMethod,
        setStepPlanOnce,
      }}
    >
      {children}
    </LocationCardContext.Provider>
  );
};
