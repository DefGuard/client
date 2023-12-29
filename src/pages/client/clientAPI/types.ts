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
export type SaveTunnelRequest = {
  privateKey: string;
  response: CreateDeviceResponse;
};

export type TrayIconTheme = 'color' | 'white' | 'black' | 'gray';

export type LogLevel = 'error' | 'info' | 'debug' | 'trace';

export type LogItemField = {
  message: string;
  interface_name?: string;
};

export type LogItem = {
  // datetime UTC
  timestamp: string;
  level: LogLevel;
  target: string;
  fields: LogItemField;
};

export type InterfaceLogsRequest = {
  locationId: DefguardLocation['id'];
};

export type Settings = {
  theme: ThemeKey;
  log_level: LogLevel;
  tray_icon_theme: TrayIconTheme;
};

export type LocationDetails = {
  location_id: number;
  name: string;
  pubkey: string;
  address: string;
  dns?: string;
  listen_port: number;
  peer_pubkey: string;
  peer_endpoint: string;
  allowed_ips: string;
  persistent_keepalive_interval?: number;
  last_handshake?: number;
};

export type LocationDetailsRequest = {
  locationId: number;
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
  | 'update_instance'
  | 'parse_tunnel_config'
  | 'save_tunnel'
  | 'all_tunnels'
  | 'location_interface_details';
