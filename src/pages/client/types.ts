export type DefguardInstance = {
  id: number;
  uuid: string;
  name: string;
  url: string;
  connected: boolean;
  pubkey: string;
};

export type DefguardLocation = {
  id: number;
  instance_id: number;
  name: string;
  address: string;
  endpoint: string;
  // connected
  active: boolean;
  route_all_traffic: boolean;
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

export enum ClientView {
  GRID = 0,
  DETAIL = 1,
}

export enum TauriEventKey {
  SINGLE_INSTANCE = 'single-instance',
  CONNECTION_CHANGED = 'connection-changed',
  INSTANCE_UPDATE = 'instance-update',
  LOCATION_UPDATE = 'location-update',
}
