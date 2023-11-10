import { CreateDeviceResponse } from '../../../shared/hooks/api/types';
import { DefguardInstance, DefguardLocation } from '../types';

export type GetLocationsRequest = {
  instanceId: number;
};

export type ConnectionRequest = {
  locationId: number;
};

export type StatsRequest = {
  locationId: number;
  from?: string;
};

export type SaveConfigRequest = {
  privateKey: string;
  response: CreateDeviceResponse;
};

export type SaveDeviceConfigResponse = {
  instance: DefguardInstance;
  locations: DefguardLocation[];
};

export type TauriCommandKey =
  | 'all_instances'
  | 'all_locations'
  | 'connect'
  | 'disconnect'
  | 'location_stats'
  | 'last_connection'
  | 'all_connections'
  | 'active_connection'
  | 'save_device_config';
