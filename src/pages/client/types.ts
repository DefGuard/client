export type DefguardInstance = {
  id: string;
  name: string;
  url: string;
  locations: DefguardLocation[];
};

export type DefguardLocation = {
  id: string;
  ip: string;
  name: string;
  connected: boolean;
};
