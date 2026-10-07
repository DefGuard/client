import { queryOptions, skipToken } from '@tanstack/react-query';

import { isPresent } from '../utils/isPresent';
import { api } from './api';
import type {
  ConnectionArgs,
  InstanceInfo,
  LocationDetailsArgs,
  StatsArgs,
} from './types';

/**
 * Single source of truth for the OR-across-instances "tunnels disabled" rule:
 * tunnels are disabled when any enrolled instance has the flag set.
 */
export const tunnelsDisabled = (instances: InstanceInfo[]): boolean =>
  instances.some((i) => i.disable_tunnels);

export const getAllActiveConnectionQueryOptions = queryOptions({
  queryKey: ['alive-connections'] as const,
  queryFn: api.getAllActiveConnections,
  refetchInterval: 5_000,
});

export const getInstancesQueryOptions = queryOptions({
  queryKey: ['instances'] as const,
  queryFn: () => api.getInstances(),
  refetchInterval: 30_000,
});

// Accepts an absent instance id and self-disables via skipToken, so callers
// (e.g. a tunnel selection with no backing instance) can pass through whatever
// they resolved without a sentinel id or a separate `enabled` flag.
export const getLocationsQueryOptions = (instanceId: number | undefined) =>
  queryOptions({
    queryKey: ['locations', instanceId] as const,
    queryFn: isPresent(instanceId) ? () => api.getLocations(instanceId) : skipToken,
  });

export const hasAnyVisibleLocationsQueryOptions = queryOptions({
  queryKey: ['has-any-visible-locations'] as const,
  queryFn: () => api.hasAnyVisibleLocations(),
});

export const getLocationDetailsQueryOptions = (args: LocationDetailsArgs) =>
  queryOptions({
    queryKey: ['location-details', args.locationId, args.connectionType] as const,
    queryFn: () => api.getLocationDetails(args),
  });

export const getLastConnectionQueryOptions = (args: ConnectionArgs) =>
  queryOptions({
    queryKey: ['last-connection', args.locationId, args.connectionType] as const,
    queryFn: () => api.getLastConnection(args),
  });

export const getConnectionHistoryQueryOptions = (args: ConnectionArgs) =>
  queryOptions({
    queryKey: ['connection-history', args.locationId, args.connectionType] as const,
    queryFn: () => api.getConnectionHistory(args),
  });

export const getActiveConnectionQueryOptions = (args: ConnectionArgs) =>
  queryOptions({
    queryKey: ['active-connection', args.locationId, args.connectionType] as const,
    queryFn: () => api.getActiveConnection(args),
  });

export const getLocationStatsQueryOptions = (args: StatsArgs) =>
  queryOptions({
    queryKey: [
      'location-stats',
      args.locationId,
      args.connectionType,
      args.from,
    ] as const,
    queryFn: () => api.getLocationStats(args),
    refetchInterval: 10_000,
  });

export const getTunnelsQueryOptions = queryOptions({
  queryKey: ['tunnels'] as const,
  queryFn: () => api.getTunnels(),
});

export const getTunnelDetailsQueryOptions = (tunnelId: number) =>
  queryOptions({
    queryKey: ['tunnel-details', tunnelId] as const,
    queryFn: () => api.getTunnelDetails(tunnelId),
  });

export const getAppConfigQueryOptions = queryOptions({
  queryKey: ['settings'] as const,
  queryFn: () => api.getAppConfig(),
});

export const getLatestAppVersionQueryOptions = queryOptions({
  queryKey: ['latest-app-version'] as const,
  queryFn: () => api.getLatestAppVersion(),
});

export const getProvisioningConfigQueryOptions = queryOptions({
  queryKey: ['provisioning-config'] as const,
  queryFn: () => api.getProvisioningConfig(),
});

export const getPlatformHeaderQueryOptions = queryOptions({
  queryKey: ['platform-header'] as const,
  queryFn: () => api.getPlatformHeader(),
});

export const getSessionStateQueryOptions = queryOptions({
  queryKey: ['session-state'] as const,
  queryFn: () => api.getSessionState(),
});
