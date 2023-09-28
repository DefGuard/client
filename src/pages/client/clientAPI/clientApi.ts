import { invoke } from '@tauri-apps/api';

import { DefguardInstance, DefguardLocation, LocationStats } from '../types';

const getInstances = async (): Promise<DefguardInstance[]> => invoke('all_instances');

type GetLocationsRequest = {
  instanceId: number;
};

const getLocations = async (data: GetLocationsRequest): Promise<DefguardLocation[]> =>
  invoke('all_locations', data);

type ConnectionRequest = {
  locationId: number;
};

const connect = async (data: ConnectionRequest): Promise<void> => invoke('connect', data);

const disconnect = async (data: ConnectionRequest): Promise<void> =>
  invoke('disconnect', data);

const getLocationStats = async (data: ConnectionRequest): Promise<LocationStats[]> =>
  invoke('location_stats', data);

export const clientApi = {
  getInstances,
  getLocations,
  connect,
  disconnect,
  getLocationStats,
};
