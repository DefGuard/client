export type DefguardInstance = {
  id: number;
  uuid: string;
  name: string;
  url: string;
  proxy_url: string;
  // connected
  active: boolean;
  pubkey: string;
  disable_all_traffic: boolean;
  openid_display_name?: string;
};

export enum LocationMfaType {
  DISABLED = 'disabled',
  INTERNAL = 'internal',
  EXTERNAL = 'external',
}

export type DefguardLocation = {
  instance_id: number;
} & CommonWireguardFields;

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

export type Tunnel = {
  id?: number;
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
} & CommonWireguardFields;

// Common fields between Tunnel, Location and instance
// Shared between components as props to avoid component duplication
export type CommonWireguardFields = {
  id: number;
  name: string;
  address: string;
  endpoint: string;
  route_all_traffic: boolean;
  // Connected
  active: boolean;
  // Tunnel or Location
  connection_type: WireguardInstanceType;
  // Available in Location only
  location_mfa_mode?: LocationMfaType;
  pubkey: string;
  instance_id: number;
  network_id: number;
};

export enum WireguardInstanceType {
  TUNNEL = 'Tunnel',
  DEFGUARD_INSTANCE = 'Instance',
}

export type SelectedInstance = {
  id?: number;
  type: WireguardInstanceType;
};

export enum ClientConnectionType {
  LOCATION = 'Location',
  TUNNEL = 'Tunnel',
}

export enum DeadConDroppedOutReason {
  PERIODIC_VERIFICATION = 'PeriodicVerification',
  CONNECTION_VERIFICATION = 'ConnectionVerification',
}

export type DeadConDroppedPayload = {
  name: string;
  con_type: ClientConnectionType;
  peer_alive_period: number;
};

export type DeadConReconnectedPayload = {
  name: string;
  con_type: ClientConnectionType;
  peer_alive_period: number;
};

export enum TauriEventKey {
  CONNECTION_CHANGED = 'connection-changed',
  INSTANCE_UPDATE = 'instance-update',
  LOCATION_UPDATE = 'location-update',
  APP_VERSION_FETCH = 'app-version-fetch',
  CONFIG_CHANGED = 'config-changed',
  DEAD_CONNECTION_DROPPED = 'dead-connection-dropped',
  DEAD_CONNECTION_RECONNECTED = 'dead-connection-reconnected',
  APPLICATION_CONFIG_CHANGED = 'application-config-changed',
  MFA_TRIGGER = 'mfa-trigger',
  VERSION_MISMATCH = 'version-mismatch',
}
