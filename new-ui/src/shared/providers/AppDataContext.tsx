import { useQuery } from '@tanstack/react-query';
import { clone } from 'radashi';
import {
  createContext,
  type PropsWithChildren,
  useCallback,
  useContext,
  useEffect,
} from 'react';
import { api } from '../rust-api/api';
import {
  getInstancesQueryOptions,
  getSessionStateQueryOptions,
  getTunnelsQueryOptions,
} from '../rust-api/query';
import type {
  ConnectionType,
  InstanceInfo,
  LocationInfo,
  MfaMethodValue,
  OverviewViewSelection,
} from '../rust-api/types';
import type { SharedSessionStorage } from './types';

interface AppDataContextValue extends SharedSessionStorage {
  instances: InstanceInfo[];
  tunnels: LocationInfo[];
  isEmpty: boolean;
  setViewSelection: (selection: OverviewViewSelection | null) => void;
  setConnectionMethod: (
    id: number,
    connectionType: ConnectionType,
    method: MfaMethodValue,
  ) => void;
}

const AppDataContext = createContext<AppDataContextValue | null>(null);

export const useAppData = (): AppDataContextValue => {
  const ctx = useContext(AppDataContext);
  if (!ctx) throw new Error('useAppData must be used within an AppDataProvider');
  return ctx;
};

export const AppDataProvider = ({ children }: PropsWithChildren) => {
  const { data: instances = [], isPending: instancesPending } = useQuery(
    getInstancesQueryOptions,
  );
  const { data: tunnels = [], isPending: tunnelsPending } =
    useQuery(getTunnelsQueryOptions);
  const { data: sessionState } = useQuery(getSessionStateQueryOptions);
  const isEmpty = instances.length === 0 && tunnels.length === 0;

  const setViewSelection = useCallback((selection: OverviewViewSelection | null) => {
    api.patchSessionState({ view_selection: selection });
  }, []);

  // keep the selection valid when its instance/tunnel gets removed
  useEffect(() => {
    if (instancesPending || tunnelsPending || sessionState === undefined) return;

    const selection = sessionState.view_selection ?? null;
    const isValid =
      selection !== null &&
      (selection.kind === 'instance'
        ? instances.some((instance) => instance.id === selection.id)
        : tunnels.some((tunnel) => tunnel.id === selection.id));

    if (isValid) return;

    const fallback: OverviewViewSelection | null =
      instances.length > 0
        ? { kind: 'instance', id: instances[0].id }
        : tunnels.length > 0
          ? { kind: 'tunnel', id: tunnels[0].id }
          : null;

    if (selection === null && fallback === null) return;

    setViewSelection(fallback);
  }, [
    instances,
    tunnels,
    instancesPending,
    tunnelsPending,
    sessionState,
    setViewSelection,
  ]);

  const setConnectionMethod = useCallback(
    (id: number, conType: ConnectionType, method: MfaMethodValue) => {
      const cloned = clone(sessionState?.connection_mfa_method ?? {});
      const key = `${conType.toLowerCase()}-${id}`;
      cloned[key] = method;
      api.patchSessionState({ connection_mfa_method: cloned });
    },
    [sessionState?.connection_mfa_method],
  );

  return (
    <AppDataContext.Provider
      value={{
        instances,
        tunnels,
        isEmpty,
        viewSelection: sessionState?.view_selection ?? null,
        connectionMfaMethod: sessionState?.connection_mfa_method ?? {},
        setConnectionMethod,
        setViewSelection,
      }}
    >
      {children}
    </AppDataContext.Provider>
  );
};
