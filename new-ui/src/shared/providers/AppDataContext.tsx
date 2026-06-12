import { useQuery } from '@tanstack/react-query';
import { createContext, type PropsWithChildren, useContext } from 'react';
import { getInstancesQueryOptions, getTunnelsQueryOptions } from '../rust-api/query';
import type { InstanceInfo, LocationInfo } from '../rust-api/types';

interface AppDataContextValue {
  instances: InstanceInfo[];
  tunnels: LocationInfo[];
  isEmpty: boolean;
}

const AppDataContext = createContext<AppDataContextValue | null>(null);

export const useAppData = (): AppDataContextValue => {
  const ctx = useContext(AppDataContext);
  if (!ctx) throw new Error('useAppData must be used within an AppDataProvider');
  return ctx;
};

export const AppDataProvider = ({ children }: PropsWithChildren) => {
  const { data: instances = [] } = useQuery(getInstancesQueryOptions);
  const { data: tunnels = [] } = useQuery(getTunnelsQueryOptions);
  const isEmpty = instances.length === 0 && tunnels.length === 0;
  return (
    <AppDataContext.Provider value={{ instances, tunnels, isEmpty }}>
      {children}
    </AppDataContext.Provider>
  );
};
