import type { InvokeArgs } from '@tauri-apps/api/core';
import { invoke } from '@tauri-apps/api/core';
import { debug, error, trace } from '@tauri-apps/plugin-log';
import pTimeout, { TimeoutError } from 'p-timeout';

import type { NewApplicationVersionInfo } from '../../../shared/hooks/api/types';
import type {
  CommonWireguardFields,
  Connection,
  DefguardInstance,
  LocationStats,
  Tunnel,
} from '../types';
import type {
  AppConfig,
  ConnectionRequest,
  GetLocationsRequest,
  LocationDetails,
  LocationDetailsRequest,
  RoutingRequest,
  SaveConfigRequest,
  SaveDeviceConfigResponse,
  StatsRequest,
  TauriCommandKey,
  TunnelRequest,
  UpdateInstanceRequest,
} from './types';

// Streamlines logging for invokes
async function invokeWrapper<T>(
  command: TauriCommandKey,
  args?: InvokeArgs,
  timeout: number = 10000,
): Promise<T> {
  debug(`Invoking "${command}" on the frontend`);
  try {
    const res = await pTimeout(invoke<T>(command, args), {
      milliseconds: timeout,
    });
    debug(`"${command}" completed on the frontend`);
    trace(`"${command}" returned: ${JSON.stringify(res)}`);
    return res;
    // TODO: handle more error types ?
  } catch (e) {
    let message: string = `Invoking "${command}" failed due to unknown error: ${JSON.stringify(
      e,
    )}`;
    trace(message);
    if (e instanceof TimeoutError) {
      message = `Invoking "${command}" timed out after ${timeout / 1000} seconds`;
    }
    error(message);
    return Promise.reject(message);
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

const deleteInstance = async (id: number): Promise<void> =>
  invokeWrapper('delete_instance', { instanceId: id });

const updateInstance = async (data: UpdateInstanceRequest): Promise<void> =>
  invokeWrapper('update_instance', data);

const parseTunnelConfig = async (config: string) =>
  invokeWrapper('parse_tunnel_config', { config: config });

const saveTunnel = async (tunnel: TunnelRequest) =>
  invokeWrapper('save_tunnel', { tunnel: tunnel });

const updateTunnel = async (tunnel: TunnelRequest) =>
  invokeWrapper('update_tunnel', { tunnel: tunnel });

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

const startGlobalLogWatcher = async (): Promise<void> =>
  invokeWrapper('start_global_logwatcher');

const stopGlobalLogWatcher = async (): Promise<void> =>
  invokeWrapper('stop_global_logwatcher');

const getAppConfig = async (): Promise<AppConfig> =>
  invokeWrapper('command_get_app_config');

const setAppConfig = async (
  appConfig: Partial<AppConfig>,
  emitEvent: boolean,
): Promise<AppConfig> =>
  invokeWrapper('command_set_app_config', {
    configPatch: appConfig,
    emitEvent,
  });

export const clientApi = {
  getAppConfig,
  setAppConfig,
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
  deleteInstance,
  deleteTunnel,
  getLocationDetails,
  updateInstance,
  parseTunnelConfig,
  saveTunnel,
  updateTunnel,
  openLink,
  getTunnelDetails,
  getLatestAppVersion,
  startGlobalLogWatcher,
  stopGlobalLogWatcher,
};
