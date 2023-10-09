import { invoke } from '@tauri-apps/api';

import { Connection, DefguardInstance, DefguardLocation, LocationStats } from '../types';

const getInstances = async (): Promise<DefguardInstance[]> => invoke('all_instances');

type GetLocationsRequest = {
  instanceId: number;
};

const getLocations = async (data: GetLocationsRequest): Promise<DefguardLocation[]> =>
  invoke('all_locations', data);

type ConnectionRequest = {
  locationId: number;
};

type StatsRequest = {
  locationId: number;
  from?: string;
};

const connect = async (data: ConnectionRequest): Promise<void> => invoke('connect', data);

const disconnect = async (data: ConnectionRequest): Promise<void> =>
  invoke('disconnect', data);

const getLocationStats = async (data: StatsRequest): Promise<LocationStats[]> =>
  invoke('location_stats', data);

const getLastConnection = async (data: ConnectionRequest): Promise<Connection> =>
  invoke('last_connection', data);

const getConnectionHistory = async (data: ConnectionRequest): Promise<Connection[]> =>
  invoke('all_connections', data);

const getActiveConnection = async (data: ConnectionRequest): Promise<Connection> =>
  invoke('active_connection', data);

export const clientApi = {
  getInstances,
  getLocations,
  connect,
  disconnect,
  getLocationStats,
  getLastConnection,
  getConnectionHistory,
  getActiveConnection,
};
