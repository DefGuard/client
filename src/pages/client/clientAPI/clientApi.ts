import { invoke } from '@tauri-apps/api';

import { DefguardInstance, DefguardLocation } from '../types';

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

export const clientApi = {
  getInstances,
  getLocations,
  connect,
  disconnect,
};
