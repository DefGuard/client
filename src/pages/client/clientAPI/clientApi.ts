import { invoke } from '@tauri-apps/api';
import { debug } from 'tauri-plugin-log-api';

import { Connection, DefguardInstance, DefguardLocation, LocationStats } from '../types';
import { ConnectionRequest, GetLocationsRequest, StatsRequest } from './types';

const getInstances = async (): Promise<DefguardInstance[]> => {
  debug("Invoking 'all_instances'");
  return invoke('all_instances');
};

const getLocations = async (data: GetLocationsRequest): Promise<DefguardLocation[]> => {
  debug("Invoking 'all_locations'");
  return invoke('all_locations', data);
};

const connect = async (data: ConnectionRequest): Promise<void> => {
  debug("Invoking 'connect'");
  invoke('connect', data);
};

const disconnect = async (data: ConnectionRequest): Promise<void> => {
  debug("Invoking 'disconnect'");
  return invoke('disconnect', data);
};

const getLocationStats = async (data: StatsRequest): Promise<LocationStats[]> => {
  debug("Invoking 'location_stats");
  return invoke('location_stats', data);
};

const getLastConnection = async (data: ConnectionRequest): Promise<Connection> => {
  debug("Invoking 'last_connection'");
  return invoke('last_connection', data);
};

const getConnectionHistory = async (data: ConnectionRequest): Promise<Connection[]> => {
  debug("Invoking 'all_connections");
  return invoke('all_connections', data);
};

const getActiveConnection = async (data: ConnectionRequest): Promise<Connection> => {
  debug("Invoking 'active_connection'");
  return invoke('active_connection', data);
};

export const clientApi = {
  getInstances,
  getLocations,
  connect,
  disconnect,
  getLocationStats,
  getLastConnection,
  getConnectionHistory,
  getActiveConnection,
};
