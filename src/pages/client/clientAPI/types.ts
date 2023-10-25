
export type GetLocationsRequest = {
  instanceId: number;
};

export type ConnectionRequest = {
  locationId: number;
};

export type StatsRequest = {
  locationId: number;
  from?: string;
};
