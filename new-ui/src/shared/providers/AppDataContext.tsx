import { useQuery, useQueryClient } from '@tanstack/react-query';
import { createContext, type PropsWithChildren, useCallback, useContext } from 'react';
import { api } from '../rust-api/api';
import {
  getInstancesQueryOptions,
  getSessionStateQueryOptions,
  getTunnelsQueryOptions,
} from '../rust-api/query';
import type {
  InstanceInfo,
  LocationInfo,
  OverviewViewSelection,
} from '../rust-api/types';
import type { SharedSessionStorage } from './types';

interface AppDataContextValue extends SharedSessionStorage {
  instances: InstanceInfo[];
  tunnels: LocationInfo[];
  isEmpty: boolean;
  setViewSelection: (selection: OverviewViewSelection | null) => void;
}

const AppDataContext = createContext<AppDataContextValue | null>(null);

export const useAppData = (): AppDataContextValue => {
  const ctx = useContext(AppDataContext);
  if (!ctx) throw new Error('useAppData must be used within an AppDataProvider');
  return ctx;
};

export const AppDataProvider = ({ children }: PropsWithChildren) => {
  const queryClient = useQueryClient();
  const { data: instances = [] } = useQuery(getInstancesQueryOptions);
  const { data: tunnels = [] } = useQuery(getTunnelsQueryOptions);
  const { data: sessionState } = useQuery(getSessionStateQueryOptions);
  const isEmpty = instances.length === 0 && tunnels.length === 0;

  const setViewSelection = useCallback(
    (selection: OverviewViewSelection | null) => {
      api
        .patchSessionState({ view_selection: selection })
        .then(() => queryClient.invalidateQueries({ queryKey: ['session-state'] }));
    },
    [queryClient],
  );

  return (
    <AppDataContext.Provider
      value={{
        instances,
        tunnels,
        isEmpty,
        viewSelection: sessionState?.view_selection ?? null,
        setViewSelection,
      }}
    >
      {children}
    </AppDataContext.Provider>
  );
};
