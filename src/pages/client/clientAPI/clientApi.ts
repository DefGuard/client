import { invoke } from '@tauri-apps/api';
import { InvokeArgs } from '@tauri-apps/api/tauri';
import pTimeout from 'p-timeout';
import { debug, error, trace } from 'tauri-plugin-log-api';

import { Connection, DefguardInstance, DefguardLocation, LocationStats } from '../types';
import {
  ConnectionRequest,
  GetLocationsRequest,
  RoutingRequest,
  SaveConfigRequest,
  SaveDeviceConfigResponse,
  StatsRequest,
  TauriCommandKey,
} from './types';

// Streamlines logging for invokes
async function invokeWrapper<T>(
  command: TauriCommandKey,
  args?: InvokeArgs,
  timeout: number = 5000,
): Promise<T> {
  console.log(`Invoking command ${command}`);
  debug(`Invoking command '${command}'`);
  try {
    const res = await pTimeout(invoke<T>(command, args), {
      milliseconds: timeout,
    });
    debug(`Invoke ${command} completed`);
    trace(`${command} completed with data: ${JSON.stringify(res)}`);
    return res;
  } catch (e) {
    error(`Invoking ${command} FAILED\n${JSON.stringify(e)}`);
    return Promise.reject(e);
  }
}

const saveConfig = async (data: SaveConfigRequest): Promise<SaveDeviceConfigResponse> =>
  invokeWrapper('save_device_config', data);

const getInstances = async (): Promise<DefguardInstance[]> =>
  invokeWrapper('all_instances');

const getLocations = async (data: GetLocationsRequest): Promise<DefguardLocation[]> =>
  invokeWrapper('all_locations', data);

const connect = async (data: ConnectionRequest): Promise<void> =>
  invokeWrapper('connect', data);

const disconnect = async (data: ConnectionRequest): Promise<void> =>
  invokeWrapper('disconnect', data);

const getLocationStats = async (data: StatsRequest): Promise<LocationStats[]> =>
  invokeWrapper('location_stats', data);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const getLocationInterfaceLogs = async (data: any): Promise<string> =>
  invokeWrapper('get_interface_logs', data);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const stopLocationInterfaceLogs = async (data: any): Promise<void> =>
  invokeWrapper('stop_interface_logs', data);

const getLastConnection = async (data: ConnectionRequest): Promise<Connection> =>
  invokeWrapper('last_connection', data);

const getConnectionHistory = async (data: ConnectionRequest): Promise<Connection[]> =>
  invokeWrapper('all_connections', data);

const getActiveConnection = async (data: ConnectionRequest): Promise<Connection> =>
  invokeWrapper('active_connection', data);

const updateLocationRouting = async (data: RoutingRequest): Promise<Connection> =>
  invokeWrapper('update_location_routing', data);

export const clientApi = {
  getInstances,
  getLocations,
  connect,
  disconnect,
  getLocationStats,
  getLocationInterfaceLogs,
  stopLocationInterfaceLogs,
  getLastConnection,
  getConnectionHistory,
  getActiveConnection,
  saveConfig,
  updateLocationRouting,
};
