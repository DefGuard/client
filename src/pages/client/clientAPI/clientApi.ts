import { invoke } from '@tauri-apps/api';
import { InvokeArgs } from '@tauri-apps/api/tauri';
import pTimeout from 'p-timeout';
import { debug, error, trace } from 'tauri-plugin-log-api';

import { NewApplicationVersionInfo } from '../../../shared/hooks/api/types';
import {
  CommonWireguardFields,
  Connection,
  DefguardInstance,
  LocationStats,
  Tunnel,
} from '../types';
import {
  ConnectionRequest,
  GetLocationsRequest,
  LocationDetails,
  LocationDetailsRequest,
  RoutingRequest,
  SaveConfigRequest,
  SaveDeviceConfigResponse,
  Settings,
  StatsRequest,
  TauriCommandKey,
  TunnelRequest,
  UpdateInstnaceRequest,
} from './types';

// Streamlines logging for invokes
async function invokeWrapper<T>(
  command: TauriCommandKey,
  args?: InvokeArgs,
  timeout: number = 5000,
): Promise<T> {
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

const getLocations = async (
  data: GetLocationsRequest,
): Promise<CommonWireguardFields[]> => invokeWrapper('all_locations', data);

const connect = async (data: ConnectionRequest): Promise<void> =>
  invokeWrapper('connect', data);

const disconnect = async (data: ConnectionRequest): Promise<void> =>
  invokeWrapper('disconnect', data);

const getLocationStats = async (data: StatsRequest): Promise<LocationStats[]> =>
  invokeWrapper('location_stats', data);

const getLastConnection = async (data: ConnectionRequest): Promise<Connection> =>
  invokeWrapper('last_connection', data);

const getConnectionHistory = async (data: ConnectionRequest): Promise<Connection[]> =>
  invokeWrapper('all_connections', data);

const getActiveConnection = async (data: ConnectionRequest): Promise<Connection> =>
  invokeWrapper('active_connection', data);

const updateLocationRouting = async (data: RoutingRequest): Promise<Connection> =>
  invokeWrapper('update_location_routing', data);

const getSettings = async (): Promise<Settings> => invokeWrapper('get_settings');

const updateSettings = async (data: Partial<Settings>): Promise<Settings> =>
  invokeWrapper('update_settings', { data });

const deleteInstance = async (id: number): Promise<void> =>
  invokeWrapper('delete_instance', { instanceId: id });

const updateInstance = async (data: UpdateInstnaceRequest): Promise<void> =>
  invokeWrapper('update_instance', data);

const parseTunnelConfig = async (config: string) =>
  invokeWrapper('parse_tunnel_config', { config: config });

const saveTunnel = async (tunnel: TunnelRequest) =>
  invokeWrapper('save_tunnel', { tunnel: tunnel });

const getLocationDetails = async (
  data: LocationDetailsRequest,
): Promise<LocationDetails> => invokeWrapper('location_interface_details', data);

const getTunnels = async (): Promise<CommonWireguardFields[]> =>
  invokeWrapper('all_tunnels');

// opens given link in system default browser
const openLink = async (link: string): Promise<void> =>
  invokeWrapper('open_link', { link });

const getTunnelDetails = async (id: number): Promise<Tunnel> =>
  invokeWrapper('tunnel_details', { tunnelId: id });

const deleteTunnel = async (id: number): Promise<void> =>
  invokeWrapper('delete_tunnel', { tunnelId: id });

const getLatestAppVersion = async (): Promise<NewApplicationVersionInfo> =>
  invokeWrapper('get_latest_app_version');

export const clientApi = {
  getInstances,
  getTunnels,
  getLocations,
  connect,
  disconnect,
  getLocationStats,
  getLastConnection,
  getConnectionHistory,
  getActiveConnection,
  saveConfig,
  updateLocationRouting,
  getSettings,
  updateSettings,
  deleteInstance,
  deleteTunnel,
  getLocationDetails,
  updateInstance,
  parseTunnelConfig,
  saveTunnel,
  openLink,
  getTunnelDetails,
  getLatestAppVersion,
};
