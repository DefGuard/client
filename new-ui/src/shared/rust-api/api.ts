import { invoke } from '@tauri-apps/api/core';
import { isPresent } from '../utils/isPresent';
import { mfaToApi } from '../utils/mfa';
import type {
  ActiveConnectionSummary,
  AppConfig,
  AppConfigPatch,
  Connection,
  ConnectionArgs,
  CreateDeviceResponse,
  EnrollmentMfaFinishResult,
  EnrollmentMfaStartResult,
  EnrollmentStartResult,
  InstanceInfo,
  LocationDetails,
  LocationDetailsArgs,
  LocationInfo,
  LocationStats,
  MfaMethodValue,
  MfaStartResult,
  MfaStepSession,
  MfaStepStartResult,
  NewAppVersionInfo,
  ProvisioningConfig,
  RoutingArgs,
  SaveConfigArgs,
  SaveDeviceConfigResponse,
  SessionState,
  SessionStatePatch,
  SetLocationMfaStepPlanArgs,
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

const setLocationMfaStepPlan = (args: SetLocationMfaStepPlanArgs): Promise<void> =>
  invoke(TauriCommand.SetLocationMfaStepPlan, args);

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

const closeWelcomeWindow = async () => invoke(TauriCommand.CloseWelcomeWindow);

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
): Promise<CreateDeviceResponse> =>
  invoke(TauriCommand.EnrollmentCreateDevice, { sessionId, name, pubkey });

const enrollmentActivateUser = (
  sessionId: string,
  password?: string | null,
  phoneNumber?: string | null,
): Promise<void> =>
  invoke(TauriCommand.EnrollmentActivateUser, { sessionId, password, phoneNumber });

const enrollmentRegisterMfaStart = (
  sessionId: string,
  method: MfaMethodValue,
): Promise<EnrollmentMfaStartResult> =>
  invoke(TauriCommand.EnrollmentRegisterMfaStart, {
    sessionId,
    method: mfaToApi(method),
  });

const enrollmentRegisterMfaFinish = (
  sessionId: string,
  code: string,
  method: MfaMethodValue,
): Promise<EnrollmentMfaFinishResult> =>
  invoke(TauriCommand.EnrollmentRegisterMfaFinish, {
    sessionId,
    code,
    method: mfaToApi(method),
  });

const enrollmentNetworkInfo = (
  sessionId: string,
  pubkey: string,
): Promise<CreateDeviceResponse> =>
  invoke(TauriCommand.EnrollmentNetworkInfo, { sessionId, pubkey });

const enrollmentFinish = (sessionId: string): Promise<void> =>
  invoke(TauriCommand.EnrollmentFinish, { sessionId });

// MFA (connect-time)

const mfaStart = (
  instanceId: number,
  locationId: number,
  methods: MfaMethodValue[],
): Promise<MfaStartResult> =>
  invoke(TauriCommand.MfaStart, { instanceId, locationId, methods });

const mfaStepStart = (
  instanceId: number,
  token: string,
  method: MfaMethodValue,
): Promise<MfaStepStartResult> =>
  invoke(TauriCommand.MfaStepStart, { instanceId, token, method });

const mfaFinishCode = (
  instanceId: number,
  locationId: number,
  token: string,
  code: string,
  stepAttemptId: string | null,
): Promise<number | null> =>
  invoke(TauriCommand.MfaFinishCode, {
    instanceId,
    locationId,
    token,
    code,
    stepAttemptId,
  });

const mfaPollOpenId = (
  instanceId: number,
  locationId: number,
  token: string,
): Promise<string> =>
  invoke(TauriCommand.MfaPollOpenId, { instanceId, locationId, token });

const mfaConnectMobileApprove = (
  instanceId: number,
  locationId: number,
  token: string,
): Promise<string> =>
  invoke(TauriCommand.MfaConnectMobileApprove, { instanceId, locationId, token });

// Hands the security key PIN to the backend, which verifies it and brings the
// connection up, the same way mfaFinishCode does for the code-based methods.
const mfaFido2Pin = (
  instanceId: number,
  locationId: number,
  pin: string,
): Promise<void> => invoke(TauriCommand.MfaFido2Pin, { instanceId, locationId, pin });

const cancelMfa = (taskId: string): Promise<void> =>
  invoke(TauriCommand.CancelMfa, { taskId });

const startMfaStep = async (
  instanceId: number,
  locationId: number,
  method: MfaMethodValue,
  stepPlan: MfaMethodValue[],
  mfaToken: string | null,
): Promise<MfaStepSession> => {
  if (isPresent(mfaToken) && stepPlan.length > 1) {
    const startedStep = await mfaStepStart(instanceId, mfaToken, method);
    return {
      token: mfaToken,
      challenge: startedStep.challenge,
      stepAttemptId: startedStep.step_attempt_id,
    };
  }

  const startedSession = await mfaStart(instanceId, locationId, stepPlan);
  return {
    token: startedSession.token,
    challenge: startedSession.challenge,
    stepAttemptId: null,
  };
};

export const api = {
  closeWelcomeWindow,
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
  setLocationMfaStepPlan,
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
  mfaFido2Pin,
  cancelMfa,
  startMfaStep,
};
