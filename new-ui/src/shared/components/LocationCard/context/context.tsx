import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react';
import { useAppData } from '../../../providers/AppDataContext';
import { api } from '../../../rust-api/api';
import type { InstanceInfo, LocationInfo } from '../../../rust-api/types';
import { ConnectionType, MfaMethod, type MfaMethodValue } from '../../../rust-api/types';
import { useAppStore } from '../../../store/useAppStore';
import { isPresent } from '../../../utils/isPresent';
import { LocationCardViews, type LocationCardViewsValue } from './types';

interface LocationCardContextValue {
  location: LocationInfo;
  instance?: InstanceInfo;
  currentView: LocationCardViewsValue;
  previousView: LocationCardViewsValue | null;
  postureError: string | null;
  autoConnectOpenid: boolean;
  mfaMethod: MfaMethodValue;
  setMfaMethod: (value: MfaMethodValue) => void;
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

  // Other location updates must not undo an optimistic connection transition.
  // biome-ignore lint/correctness/useExhaustiveDependencies: synchronize only on active state
  useEffect(() => {
    if (location.active) {
      setCurrentView(LocationCardViews.Connected);
    } else {
      setMfaMethod(location.mfa_method ?? MfaMethod.Totp);
      setCurrentView(LocationCardViews.Default);
    }
  }, [location.active]);

  const setView = useCallback(
    (view: LocationCardViewsValue) => {
      setPreviousView(currentView);
      setCurrentView(view);
    },
    [currentView],
  );

  const startMfa = useCallback(async () => {
    mfaStarted.current = true;
    const appConfig = await api.getAppConfig();
    setAutoConnectOpenid(appConfig.auto_start_openid_mfa);
    switch (mfaMethod) {
      case MfaMethod.Totp:
        setView(LocationCardViews.MfaTotp);
        break;
      case MfaMethod.Email:
        setView(LocationCardViews.MfaEmail);
        break;
      case MfaMethod.Oidc:
        setView(LocationCardViews.MfaOidc);
        break;
      case MfaMethod.MobileApprove:
        setView(LocationCardViews.MfaMobile);
        break;
      case MfaMethod.Fido2:
        setView(LocationCardViews.MfaFido2);
        break;
    }
  }, [setView, mfaMethod]);

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
        setView,
        setPostureError,
        startMfa,
        setMfaMethod,
      }}
    >
      {children}
    </LocationCardContext.Provider>
  );
};
