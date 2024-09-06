export type DefguardInstance = {
  id: number;
  uuid: string;
  name: string;
  url: string;
  proxy_url: string;
  // connected
  active: boolean;
  pubkey: string;
};

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
  mfa_enabled: boolean | undefined;
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

export enum TauriEventKey {
  SINGLE_INSTANCE = 'single-instance',
  CONNECTION_CHANGED = 'connection-changed',
  INSTANCE_UPDATE = 'instance-update',
  LOCATION_UPDATE = 'location-update',
  APP_VERSION_FETCH = 'app-version-fetch',
  CONFIG_CHANGED = 'config-changed',
}
