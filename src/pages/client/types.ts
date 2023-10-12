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
