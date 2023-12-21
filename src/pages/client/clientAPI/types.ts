import { ThemeKey } from '../../../shared/defguard-ui/hooks/theme/types';
import { CreateDeviceResponse } from '../../../shared/hooks/api/types';
import { DefguardInstance, DefguardLocation } from '../types';

export type GetLocationsRequest = {
  instanceId: number;
};

export type ConnectionRequest = {
  locationId: number;
};

export type RoutingRequest = {
  locationId: number;
  routeAllTraffic?: boolean;
};

export type StatsRequest = {
  locationId: number;
  from?: string;
};

export type SaveConfigRequest = {
  privateKey: string;
  response: CreateDeviceResponse;
};

export type UpdateInstnaceRequest = {
  instanceId: number;
  response: CreateDeviceResponse;
};

export type SaveDeviceConfigResponse = {
  instance: DefguardInstance;
  locations: DefguardLocation[];
};

export type TrayIconTheme = 'color' | 'white' | 'black' | 'gray';

export type LogLevel = 'error' | 'info' | 'debug' | 'trace';

export type Settings = {
  theme: ThemeKey;
  log_level: LogLevel;
  tray_icon_theme: TrayIconTheme;
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
  | 'save_device_config'
  | 'update_location_routing'
  | 'get_settings'
  | 'update_settings'
  | 'delete_instance'
  | 'update_instance';
