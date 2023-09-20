export type DefguardInstance = {
  id: number;
  uuid: string;
  name: string;
  url: string;
};

export type DefguardLocation = {
  id: number;
  instance_id: number;
  network_id: number;
  name: string;
  address: string;
  pubkey: string;
  endpoint: string;
  allowed_ips: string;
};
