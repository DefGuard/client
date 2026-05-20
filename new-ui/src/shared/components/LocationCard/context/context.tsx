import { createContext, type ReactNode, useCallback, useContext, useState } from 'react';
import type { InstanceInfo, LocationInfo } from '../../../rust-api/types';
import { MfaMethod } from '../../../rust-api/types';
import { LocationCardViews, type LocationCardViewsValue } from './types';

interface LocationCardContextValue {
  location: LocationInfo;
  instance: InstanceInfo;
  currentView: LocationCardViewsValue;
  previousView: LocationCardViewsValue | null;
  setView: (view: LocationCardViewsValue) => void;
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
  const [previousView, setPreviousView] = useState<LocationCardViewsValue | null>(null);
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

  const startMfa = useCallback(() => {
    switch (location.mfa_method) {
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
  }, [location.mfa_method, setView]);

  return (
    <LocationCardContext.Provider
      value={{
        currentView,
        previousView,
        setView,
        location,
        instance,
        startMfa,
      }}
    >
      {children}
    </LocationCardContext.Provider>
  );
};
