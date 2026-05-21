import { getVersion } from '@tauri-apps/api/app';

import { invoke } from '@tauri-apps/api/core';

import type {
  ActiveConnectionSummary,
  AppConfig,
  AppConfigPatch,
  Connection,
  ConnectionArgs,
  EdgeRequestHeaders,
  InstanceInfo,
  LocationDetails,
  LocationDetailsArgs,
  LocationInfo,
  LocationStats,
  NewAppVersionInfo,
  ProvisioningConfig,
  RoutingArgs,
  SaveConfigArgs,
  SaveDeviceConfigResponse,
  SetLocationMfaMethodArgs,
  StatsArgs,
  TunnelInfo,
  TunnelRequest,
  UpdateInstanceArgs,
} from './types';
import { TauriCommand } from './types';

const getInstances = (): Promise<InstanceInfo[]> => invoke(TauriCommand.AllInstances);

const deleteInstance = (instanceId: number): Promise<void> =>
  invoke(TauriCommand.DeleteInstance, { instanceId });

const updateInstance = (args: UpdateInstanceArgs): Promise<void> =>
  invoke(TauriCommand.UpdateInstance, args);

const saveDeviceConfig = (args: SaveConfigArgs): Promise<SaveDeviceConfigResponse> =>
  invoke(TauriCommand.SaveDeviceConfig, args);

const getLocations = (instanceId: number): Promise<LocationInfo[]> =>
  invoke(TauriCommand.AllLocations, { instanceId });

const getLocationDetails = (args: LocationDetailsArgs): Promise<LocationDetails> =>
  invoke(TauriCommand.LocationInterfaceDetails, args);

const updateLocationRouting = (args: RoutingArgs): Promise<Connection> =>
  invoke(TauriCommand.UpdateLocationRouting, args);

const setLocationMfaMethod = (args: SetLocationMfaMethodArgs): Promise<void> =>
  invoke(TauriCommand.SetLocationMfaMethod, args);

const connect = (args: ConnectionArgs): Promise<void> =>
  invoke(TauriCommand.Connect, args);

const disconnect = (args: ConnectionArgs): Promise<void> =>
  invoke(TauriCommand.Disconnect, args);

const getLastConnection = (args: ConnectionArgs): Promise<Connection> =>
  invoke(TauriCommand.LastConnection, args);

const getConnectionHistory = (args: ConnectionArgs): Promise<Connection[]> =>
  invoke(TauriCommand.AllConnections, args);

const getActiveConnection = (args: ConnectionArgs): Promise<Connection> =>
  invoke(TauriCommand.ActiveConnection, args);

const getLocationStats = (args: StatsArgs): Promise<LocationStats[]> =>
  invoke(TauriCommand.LocationStats, args);

const getTunnels = (): Promise<LocationInfo[]> => invoke(TauriCommand.AllTunnels);

const getTunnelDetails = (tunnelId: number): Promise<TunnelInfo> =>
  invoke(TauriCommand.TunnelDetails, { tunnelId });

const parseTunnelConfig = (
  filename: string,
  config: string,
): Promise<Partial<TunnelRequest>> =>
  invoke(TauriCommand.ParseTunnelConfig, { filename, config });

const saveTunnel = (tunnel: TunnelRequest): Promise<void> =>
  invoke(TauriCommand.SaveTunnel, { tunnel });

const updateTunnel = (tunnel: TunnelRequest): Promise<void> =>
  invoke(TauriCommand.UpdateTunnel, { tunnel });

const deleteTunnel = (tunnelId: number): Promise<void> =>
  invoke(TauriCommand.DeleteTunnel, { tunnelId });

const getAppConfig = (): Promise<AppConfig> => invoke(TauriCommand.GetAppConfig);

const setAppConfig = (
  configPatch: AppConfigPatch,
  emitEvent: boolean,
): Promise<AppConfig> => invoke(TauriCommand.SetAppConfig, { configPatch, emitEvent });

const getProvisioningConfig = (): Promise<ProvisioningConfig | null> =>
  invoke(TauriCommand.GetProvisioningConfig);

const getPlatformHeader = (): Promise<string> => invoke(TauriCommand.GetPlatformHeader);

const getLatestAppVersion = (): Promise<NewAppVersionInfo> =>
  invoke(TauriCommand.GetLatestAppVersion);

const openLink = (link: string): Promise<void> => invoke(TauriCommand.OpenLink, { link });

const startGlobalLogWatcher = (): Promise<void> =>
  invoke(TauriCommand.StartGlobalLogWatcher);

const stopGlobalLogWatcher = (): Promise<void> =>
  invoke(TauriCommand.StopGlobalLogWatcher);

const getAllActiveConnections = (): Promise<ActiveConnectionSummary[]> =>
  invoke(TauriCommand.AllActiveConnections);

const disconnectLocations = (locationIds: number[]): Promise<void> =>
  invoke(TauriCommand.DisconnectLocations, { locationIds });

const getEdgeRequestHeaders = async (): Promise<EdgeRequestHeaders> => {
  const platform = await getPlatformHeader();
  const version = await getVersion().catch(() => 'unknown');
  return {
    'defguard-client-platform': platform,
    'defguard-client-version': version,
  };
};

const getPostureData = async (): Promise<unknown> =>
  invoke(TauriCommand.GetPostureData);

const swapToOldUi = async () => invoke(TauriCommand.SwapToOldUi);

export const api = {
  getEdgeRequestHeaders,
  // Instances
  getInstances,
  deleteInstance,
  updateInstance,
  saveDeviceConfig,
  // Locations
  getLocations,
  getLocationDetails,
  updateLocationRouting,
  setLocationMfaMethod,
  // Connections
  connect,
  disconnect,
  getLastConnection,
  getConnectionHistory,
  getActiveConnection,
  getLocationStats,
  // Tunnels
  getTunnels,
  getTunnelDetails,
  parseTunnelConfig,
  saveTunnel,
  updateTunnel,
  deleteTunnel,
  // App config
  getAppConfig,
  setAppConfig,
  // Misc
  getProvisioningConfig,
  getPlatformHeader,
  getLatestAppVersion,
  openLink,
  startGlobalLogWatcher,
  stopGlobalLogWatcher,
  getAllActiveConnections,
  disconnectLocations,
  getPostureData,
  // Window
  swapToOldUi,
};
