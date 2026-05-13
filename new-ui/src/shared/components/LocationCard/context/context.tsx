import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useState,
} from 'react';
import { useCompactLocationStore } from '../../../../pages/compact/CompactLocationsPage/hooks/useCompactLocationsStore';
import type { LocationInfo } from '../../../rust-api/types';
import { MfaMethod, type MfaMethodValue } from '../../../rust-api/types';
import { LocationCardViews, type LocationCardViewsValue } from './types';

interface LocationCardContextValue {
  currentView: LocationCardViewsValue;
  previousView: LocationCardViewsValue | null;
  setView: (view: LocationCardViewsValue) => void;
  location: LocationInfo;
  mfaMethod: MfaMethodValue | null;
  setMfaMethod: (method: MfaMethodValue | null) => void;
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
  location: LocationInfo;
  children: ReactNode;
}

export const LocationCardProvider = ({
  location,
  children,
}: LocationCardProviderProps) => {
  const [previousView, setPreviousView] = useState<LocationCardViewsValue | null>(null);
  const [currentView, setCurrentView] = useState<LocationCardViewsValue>(
    LocationCardViews.Default,
  );

  const [mfaMethod, setMfaMethod] = useState<MfaMethodValue | null>(null);

  const setView = useCallback(
    (view: LocationCardViewsValue) => {
      setPreviousView(currentView);
      setCurrentView(view);
    },
    [currentView],
  );

  const startMfa = useCallback(() => {
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
  }, [mfaMethod, setView]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: should only have mfaMethod
  useEffect(() => {
    useCompactLocationStore.getState().setLocationMfa(location.id, mfaMethod);
  }, [mfaMethod]);

  return (
    <LocationCardContext.Provider
      value={{
        currentView,
        previousView,
        setView,
        location: location,
        mfaMethod,
        setMfaMethod,
        startMfa,
      }}
    >
      {children}
    </LocationCardContext.Provider>
  );
};
