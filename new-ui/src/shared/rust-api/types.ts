export const AppTheme = {
  Light: 'light',
  Dark: 'dark',
} as const;

export type AppThemeValue = (typeof AppTheme)[keyof typeof AppTheme];

export const AppTrayTheme = {
  Color: 'color',
  White: 'white',
  Black: 'black',
  Gray: 'gray',
} as const;

export type AppTrayTheme = (typeof AppTrayTheme)[keyof typeof AppTrayTheme];

export const LogLevel = {
  Off: 'OFF',
  Error: 'ERROR',
  Warn: 'WARN',
  Info: 'INFO',
  Debug: 'DEBUG',
  Trace: 'TRACE',
} as const;

export type LogLevel = (typeof LogLevel)[keyof typeof LogLevel];

export const LogSource = {
  All: 'All',
  Client: 'Client',
  Vpn: 'VPN',
} as const;

export type LogSource = (typeof LogSource)[keyof typeof LogSource];

export type LogItem = {
  // datetime UTC
  timestamp: string;
  level: LogLevel;
  target: string;
  fields: {
    message: string;
    interface_name?: string;
  };
  source: LogSource;
};

export const ClientTrafficPolicy = {
  None: 'none',
  DisableAllTraffic: 'disable_all_traffic',
  ForceAllTraffic: 'force_all_traffic',
} as const;

export type ClientTrafficPolicy =
  (typeof ClientTrafficPolicy)[keyof typeof ClientTrafficPolicy];

export const LocationMfaMode = {
  Disabled: 'disabled',
  Internal: 'internal',
  External: 'external',
} as const;

export type LocationMfaMode = (typeof LocationMfaMode)[keyof typeof LocationMfaMode];

export const MfaMethod = {
  Totp: 'totp',
  Email: 'email',
  Oidc: 'oidc',
  Biometric: 'biometric',
  MobileApprove: 'mobileapprove',
} as const;

export type MfaMethodValue = (typeof MfaMethod)[keyof typeof MfaMethod];

export const ConnectionType = {
  Location: 'Location',
  Tunnel: 'Tunnel',
} as const;

export type ConnectionType = (typeof ConnectionType)[keyof typeof ConnectionType];

/** Typed enum for every Tauri command available on the backend. */
export const TauriCommand = {
  // Instances
  AllInstances: 'all_instances',
  DeleteInstance: 'delete_instance',
  UpdateInstance: 'update_instance',
  SaveDeviceConfig: 'save_device_config',
  // Locations
  AllLocations: 'all_locations',
  HasAnyVisibleLocations: 'has_any_visible_locations',
  LocationInterfaceDetails: 'location_interface_details',
  UpdateLocationRouting: 'update_location_routing',
  SetLocationMfaMethod: 'set_location_mfa_method',
  // Connections
  Connect: 'connect',
  Disconnect: 'disconnect',
  LastConnection: 'last_connection',
  AllConnections: 'all_connections',
  ActiveConnection: 'active_connection',
  LocationStats: 'location_stats',
  // Tunnels
  AllTunnels: 'all_tunnels',
  TunnelDetails: 'tunnel_details',
  ParseTunnelConfig: 'parse_tunnel_config',
  SaveTunnel: 'save_tunnel',
  UpdateTunnel: 'update_tunnel',
  DeleteTunnel: 'delete_tunnel',
  // App config
  GetAppConfig: 'command_get_app_config',
  SetAppConfig: 'command_set_app_config',
  // Misc
  GetProvisioningConfig: 'get_provisioning_config',
  GetPlatformHeader: 'get_platform_header',
  GetLatestAppVersion: 'get_latest_app_version',
  OpenLink: 'open_link',
  StartGlobalLogWatcher: 'start_global_logwatcher',
  StopGlobalLogWatcher: 'stop_global_logwatcher',
  AllActiveConnections: 'all_active_connections',
  DisconnectLocations: 'disconnect_locations',
  GetPostureData: 'get_posture_data',
  //Window
  SwapToFullView: 'swap_to_full_view',
  SwapToTray: 'swap_to_tray',
  CloseTrayWindow: 'close_tray_window',
  // Session state
  GetSessionState: 'get_session_state',
  PatchSessionState: 'patch_session_state',
} as const;

export type TauriCommand = (typeof TauriCommand)[keyof typeof TauriCommand];

/** Typed enum for every Tauri event emitted by the backend. */
export const TauriEvent = {
  ConnectionChanged: 'connection-changed',
  InstanceUpdate: 'instance-update',
  LocationUpdate: 'location-update',
  AppVersionFetch: 'app-version-fetch',
  ConfigChanged: 'config-changed',
  DeadConnectionDropped: 'dead-connection-dropped',
  DeadConnectionReconnected: 'dead-connection-reconnected',
  ApplicationConfigChanged: 'application-config-changed',
  AddInstance: 'add-instance',
  MfaTrigger: 'mfa-trigger',
  VersionMismatch: 'version-mismatch',
  UuidMismatch: 'uuid-mismatch',
  GlobalLogUpdate: 'log-update-global',
  WindowSwapped: 'window-swapped',
  SessionStateChanged: 'session-state-changed',
} as const;

export type TauriEventValue = (typeof TauriEvent)[keyof typeof TauriEvent];

/** Payload for the `dead-connection-dropped` event. Mirrors `DeadConnDroppedOut` in events.rs. */
export type DeadConnectionDroppedPayload = {
  name: string;
  con_type: ConnectionType;
  peer_alive_period: number;
};

/** Payload for the `dead-connection-reconnected` event. Mirrors `DeadConnReconnected` in events.rs. */
export type DeadConnectionReconnectedPayload = {
  name: string;
  con_type: ConnectionType;
  peer_alive_period: number;
};

/** Payload for the `add-instance` event. Mirrors `AddInstancePayload` in events.rs. */
export type AddInstanceEventPayload = {
  token: string;
  url: string;
};

export type ActiveConnectionSummary = {
  id: number;
  name: string;
  connection_type: ConnectionType;
};

export type AppConfig = {
  theme: AppThemeValue;
  tray_theme: AppTrayTheme;
  check_for_updates: boolean;
  log_level: LogLevel;
  /** Idle seconds before the connection is automatically dropped. */
  peer_alive_period: number;
  /** Maximum transmission unit; 0 means system default. */
  mtu: number;
  auto_start_openid_mfa: boolean;
};

export type AppConfigPatch = Partial<AppConfig>;

export type InstanceInfo = {
  id: number;
  name: string;
  /** Server-side UUID (not the SQLite row id). */
  uuid: string;
  url: string;
  proxy_url: string;
  /** True when at least one location of this instance has an active connection. */
  active: boolean;
  pubkey: string;
  client_traffic_policy: ClientTrafficPolicy;
  enterprise_enabled: boolean;
  openid_display_name: string | null;
};

export type LocationInfo = {
  id: number;
  instance_id: number;
  name: string;
  address: string;
  endpoint: string;
  active: boolean;
  route_all_traffic: boolean;
  connection_type: ConnectionType;
  pubkey: string;
  network_id: number;
  location_mfa_mode: LocationMfaMode;
  mfa_method?: MfaMethodValue;
  posture_check_required: boolean;
};

export type LocationStats = {
  collected_at: number;
  download: number;
  upload: number;
};

export type Connection = {
  id: number;
  location_id: number;
  connected_from: string;
  start: string;
  end: string;
  upload?: number;
  download?: number;
};

export type TunnelInfo = {
  id?: number;
  name: string;
  address: string;
  endpoint: string;
  route_all_traffic: boolean;
  active: boolean;
  connection_type: ConnectionType;
  instance_id: number;
  network_id: number;
  pubkey: string;
  prvkey: string;
  server_pubkey: string;
  preshared_key?: string;
  allowed_ips?: string;
  dns?: string;
  persistent_keep_alive: number;
  pre_up?: string;
  post_up?: string;
  pre_down?: string;
  post_down?: string;
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
  mfa_method?: MfaMethodValue;
};

export type NewAppVersionInfo = {
  version: string;
  release_date: string;
  release_notes_url: string;
  update_url: string;
  summary?: string | null;
  notes?: string | null;
};

export type ProvisioningConfig = {
  enrollment_token: string;
  enrollment_url: string;
};

export type Device = {
  id: number;
  name: string;
  pubkey: string;
  privateKey?: string;
  user_id: number;
  created_at: number;
};

export type DeviceConfig = {
  network_id: number;
  network_name: string;
  config: string;
};

export type CreateDeviceResponse = {
  device: Device;
  configs: DeviceConfig[];
  instance: InstanceInfo;
};

export type SaveDeviceConfigResponse = {
  instance: InstanceInfo;
  locations: LocationInfo[];
};

export type ConnectionArgs = {
  locationId: number;
  connectionType: ConnectionType;
  presharedKey?: string;
};

export type RoutingArgs = {
  locationId: number;
  connectionType: ConnectionType;
  routeAllTraffic?: boolean;
};

export type StatsArgs = {
  locationId: number;
  connectionType: ConnectionType;
  from?: string;
};

export type LocationDetailsArgs = {
  locationId: number;
  connectionType: ConnectionType;
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

export type SaveConfigArgs = {
  privateKey: string;
  response: CreateDeviceResponse;
};

export type UpdateInstanceArgs = {
  instanceId: number;
  response: CreateDeviceResponse;
};

export type SetLocationMfaMethodArgs = {
  locationId: number;
  mfaMethod: MfaMethodValue;
};

export type OverviewViewSelection = {
  kind: 'instance' | 'tunnel';
  id: number;
};

export type SessionState = {
  view_selection: OverviewViewSelection | null;
};

export type SessionStatePatch = Partial<SessionState>;
