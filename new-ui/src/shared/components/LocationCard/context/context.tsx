import { createContext, type ReactNode, useCallback, useContext, useState } from 'react';
import { useAppData } from '../../../providers/AppDataContext';
import { api } from '../../../rust-api/api';
import type { InstanceInfo, LocationInfo } from '../../../rust-api/types';
import { MfaMethod } from '../../../rust-api/types';
import { decideLocationMfaMethod } from '../../../utils/decideLocationMfaMethod';
import { LocationCardViews, type LocationCardViewsValue } from './types';

interface LocationCardContextValue {
  location: LocationInfo;
  instance: InstanceInfo;
  currentView: LocationCardViewsValue;
  previousView: LocationCardViewsValue | null;
  postureError: string | null;
  autoConnectOpenid: boolean;
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
  instance: InstanceInfo;
  location: LocationInfo;
  children: ReactNode;
}

export const LocationCardProvider = ({
  location,
  instance,
  children,
}: LocationCardProviderProps) => {
  const [autoConnectOpenid, setAutoConnectOpenid] = useState(false);
  const [previousView, setPreviousView] = useState<LocationCardViewsValue | null>(null);
  const [postureError, setPostureError] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<LocationCardViewsValue>(
    location.active ? LocationCardViews.Connected : LocationCardViews.Default,
  );

  const setView = useCallback(
    (view: LocationCardViewsValue) => {
      setPreviousView(currentView);
      setCurrentView(view);
    },
    [currentView],
  );

  const { locationMfaPreference } = useAppData();

  const startMfa = useCallback(async () => {
    const appConfig = await api.getAppConfig();
    setAutoConnectOpenid(appConfig.auto_start_openid_mfa);

    const mfaMethod = decideLocationMfaMethod(
      location,
      locationMfaPreference[String(location.id)],
    );
    if (!mfaMethod) return;

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
    }
  }, [setView, location.id, locationMfaPreference, location]);

  return (
    <LocationCardContext.Provider
      value={{
        currentView,
        previousView,
        postureError,
        autoConnectOpenid,
        setView,
        setPostureError,
        location,
        instance,
        startMfa,
      }}
    >
      {children}
    </LocationCardContext.Provider>
  );
};
