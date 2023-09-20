import { invoke } from '@tauri-apps/api';

import { DefguardInstance, DefguardLocation } from '../types';

const getInstances = async (): Promise<DefguardInstance[]> => invoke('all_instances');

type GetLocationsRequest = {
  instance_id: number;
};

const getLocations = async (data: GetLocationsRequest): Promise<DefguardLocation[]> =>
  invoke('all_locations', data);

export const clientApi = () => ({
  getInstances,
  getLocations,
});
