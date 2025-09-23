import type { ThemeKey } from '../../../shared/defguard-ui/hooks/theme/types';
import type { CreateDeviceResponse } from '../../../shared/hooks/api/types';
import type { DefguardInstance, DefguardLocation, ClientConnectionType } from '../types';

export type GetLocationsRequest = {
  instanceId: number;
};

export type ConnectionRequest = {
  locationId: number;
  connectionType: ClientConnectionType;
  presharedKey?: string;
};

export type RoutingRequest = {
  locationId: number;
  connectionType: ClientConnectionType;
  routeAllTraffic?: boolean;
};

export type StatsRequest = {
  locationId: number;
  connectionType: ClientConnectionType;
  from?: string;
};

export type SaveConfigRequest = {
  privateKey: string;
  response: CreateDeviceResponse;
};

export type UpdateInstanceRequest = {
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

export const availableTrayThemes: TrayIconTheme[] = ['color', 'white', 'gray', 'black'];

export type LogLevel = 'ERROR' | 'INFO' | 'DEBUG' | 'TRACE' | 'WARN';

export const availableLogLevels: LogLevel[] = ['ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE'];

export type GlobalLogLevel = 'ERROR' | 'INFO' | 'DEBUG';
export type LogSource = 'Client' | 'Service' | 'All';

export type ClientView = 'grid' | 'detail';

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
  source: LogSource;
};

export type InterfaceLogsRequest = {
  locationId: DefguardLocation['id'];
};

export type AppConfig = {
  theme: ThemeKey;
  log_level: LogLevel;
  tray_theme: TrayIconTheme;
  check_for_updates: boolean;
  peer_alive_period: number;
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

export type TunnelRequest = {
  name: string;
  pubkey: string;
  prvkey: string;
  address: string;
  server_pubkey: string;
  allowed_ips?: string;
  endpoint: string;
  dns?: string;
  persistent_keep_alive: number;
  pre_up?: string;
  post_up?: string;
  pre_down?: string;
  post_down?: string;
};

export type LocationDetailsRequest = {
  locationId: number;
  connectionType: ClientConnectionType;
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
  | 'delete_instance'
  | 'update_instance'
  | 'parse_tunnel_config'
  | 'save_tunnel'
  | 'update_tunnel'
  | 'all_tunnels'
  | 'tunnel_details'
  | 'delete_tunnel'
  | 'location_interface_details'
  | 'open_link'
  | 'get_latest_app_version'
  | 'start_global_logwatcher'
  | 'stop_global_logwatcher'
  | 'command_get_app_config'
  | 'command_set_app_config';
