import { invoke } from '@tauri-apps/api/core';
import type {
  ActiveConnectionSummary,
  AppConfig,
  AppConfigPatch,
  Connection,
  ConnectionArgs,
  EnrollmentMfaFinishResult,
  EnrollmentMfaStartResult,
  EnrollmentStartResult,
  InstanceInfo,
  LocationDetails,
  LocationDetailsArgs,
  LocationInfo,
  LocationStats,
  MfaStartResult,
  NewAppVersionInfo,
  ProvisioningConfig,
  RoutingArgs,
  SaveConfigArgs,
  SaveDeviceConfigResponse,
  SessionState,
  SessionStatePatch,
  SetLocationMfaMethodArgs,
  StatsArgs,
  TunnelInfo,
  TunnelRequest,
  UpdateInstanceArgs,
  UpdateTunnelRequest,
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

const hasAnyVisibleLocations = (): Promise<boolean> =>
  invoke(TauriCommand.HasAnyVisibleLocations);

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

const parseTunnelConfig = (data: {
  filename: string;
  config: string;
}): Promise<Partial<TunnelRequest>> => invoke(TauriCommand.ParseTunnelConfig, data);

const saveTunnel = (tunnel: TunnelRequest): Promise<void> =>
  invoke(TauriCommand.SaveTunnel, { tunnel });

const updateTunnel = (tunnel: UpdateTunnelRequest): Promise<void> =>
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

const getPostureData = async (): Promise<unknown> => invoke(TauriCommand.GetPostureData);

const swapToFullView = async () => invoke(TauriCommand.SwapToFullView);

const swapToTray = async () => invoke(TauriCommand.SwapToTray);

const closeTrayWindow = async () => invoke(TauriCommand.CloseTrayWindow);

const getSessionState = (): Promise<SessionState> => invoke(TauriCommand.GetSessionState);

const patchSessionState = (patch: SessionStatePatch): Promise<SessionState> =>
  invoke(TauriCommand.PatchSessionState, { patch });

// Enrollment

const enrollmentStart = (
  proxyUrl: string,
  token: string,
): Promise<EnrollmentStartResult> =>
  invoke(TauriCommand.EnrollmentStart, { proxyUrl, token });

const enrollmentCreateDevice = (
  sessionId: string,
  name: string,
  pubkey: string,
): Promise<unknown> =>
  invoke(TauriCommand.EnrollmentCreateDevice, { sessionId, name, pubkey });

const enrollmentActivateUser = (
  sessionId: string,
  password?: string | null,
  phoneNumber?: string | null,
): Promise<void> =>
  invoke(TauriCommand.EnrollmentActivateUser, { sessionId, password, phoneNumber });

const enrollmentRegisterMfaStart = (
  sessionId: string,
  method: string,
): Promise<EnrollmentMfaStartResult> =>
  invoke(TauriCommand.EnrollmentRegisterMfaStart, { sessionId, method });

const enrollmentRegisterMfaFinish = (
  sessionId: string,
  code: string,
  method: string,
): Promise<EnrollmentMfaFinishResult> =>
  invoke(TauriCommand.EnrollmentRegisterMfaFinish, { sessionId, code, method });

const enrollmentNetworkInfo = (sessionId: string, pubkey: string): Promise<unknown> =>
  invoke(TauriCommand.EnrollmentNetworkInfo, { sessionId, pubkey });

const enrollmentFinish = (sessionId: string): Promise<void> =>
  invoke(TauriCommand.EnrollmentFinish, { sessionId });

// MFA (connect-time)

const mfaStart = (
  instanceId: number,
  locationId: number,
  method: string,
): Promise<MfaStartResult> =>
  invoke(TauriCommand.MfaStart, { instanceId, locationId, method });

const mfaFinishCode = (
  instanceId: number,
  token: string,
  code: string,
): Promise<{ preshared_key: string }> =>
  invoke(TauriCommand.MfaFinishCode, { instanceId, token, code });

const mfaPollOpenId = (instanceId: number, token: string): Promise<string> =>
  invoke(TauriCommand.MfaPollOpenId, { instanceId, token });

const mfaConnectMobileApprove = (instanceId: number, token: string): Promise<string> =>
  invoke(TauriCommand.MfaConnectMobileApprove, { instanceId, token });

const cancelMfa = (taskId: string): Promise<void> =>
  invoke(TauriCommand.CancelMfa, { taskId });

export const api = {
  // Instances
  getInstances,
  deleteInstance,
  updateInstance,
  saveDeviceConfig,
  // Locations
  getLocations,
  hasAnyVisibleLocations,
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
  swapToFullView,
  swapToTray,
  closeTrayWindow,
  // Session state
  getSessionState,
  patchSessionState,
  // Enrollment
  enrollmentStart,
  enrollmentCreateDevice,
  enrollmentActivateUser,
  enrollmentRegisterMfaStart,
  enrollmentRegisterMfaFinish,
  enrollmentNetworkInfo,
  enrollmentFinish,
  // MFA
  mfaStart,
  mfaFinishCode,
  mfaPollOpenId,
  mfaConnectMobileApprove,
  cancelMfa,
};
