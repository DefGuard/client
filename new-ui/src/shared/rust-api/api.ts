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
  EnrollmentStartResult,
  InstanceInfo,
  LocationDetails,
  LocationDetailsArgs,
  LocationInfo,
  LocationStats,
  MfaConfigAuthorizeResult,
  MfaConfigStartResult,
  MfaMethodValue,
  MfaSetupFinishResult,
  MfaSetupStartResult,
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
): Promise<MfaSetupStartResult> =>
  invoke(TauriCommand.EnrollmentRegisterMfaStart, {
    sessionId,
    method: mfaToApi(method),
  });

const enrollmentRegisterMfaFinish = (
  sessionId: string,
  code: string,
  method: MfaMethodValue,
): Promise<MfaSetupFinishResult> =>
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

// Starts FIDO2 verification. The backend runs it as a task - it fetches the
// challenge and credential id from Edge, drives the security key, submits the
// assertion and brings the connection up - so this resolves with the task id
// and the outcome arrives as an MfaFido2Complete / MfaFido2Error event.
const mfaFido2Pin = (
  instanceId: number,
  locationId: number,
  methods: MfaMethodValue[],
  token: string | null,
  // Null where the platform collects the PIN itself - see `fido2CollectsPinInApp`.
  pin: string | null,
): Promise<string> =>
  invoke(TauriCommand.MfaFido2Pin, { instanceId, locationId, methods, token, pin });

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

// MFA configuration

const mfaConfigStart = (instanceId: number): Promise<MfaConfigStartResult> =>
  invoke(TauriCommand.MfaConfigStart, { instanceId });

const mfaConfigSendCode = (sessionId: string): Promise<void> =>
  invoke(TauriCommand.MfaConfigSendCode, { sessionId });

const mfaConfigAuthorize = (
  sessionId: string,
  method: MfaMethodValue,
  code: string,
): Promise<MfaConfigAuthorizeResult> =>
  invoke(TauriCommand.MfaConfigAuthorize, { sessionId, method, code });

const mfaConfigSetupStart = (
  sessionId: string,
  method: MfaMethodValue,
): Promise<MfaSetupStartResult> =>
  invoke(TauriCommand.MfaConfigSetupStart, { sessionId, method });

const mfaConfigSetupFinish = (
  sessionId: string,
  method: MfaMethodValue,
  code: string,
): Promise<MfaSetupFinishResult> =>
  invoke(TauriCommand.MfaConfigSetupFinish, { sessionId, method, code });

// Challenge, key ceremony and attestation submit in one call, so it resolves only once the
// user has touched the key. `mfa-config-fido2-touch` is emitted while it waits.
const mfaConfigSetupFido2 = (
  sessionId: string,
  name: string,
  // Null where the platform collects the PIN itself - see `fido2CollectsPinInApp`.
  pin: string | null,
): Promise<MfaSetupFinishResult> =>
  invoke(TauriCommand.MfaConfigSetupFido2, { sessionId, name, pin });

const mfaConfigCancel = (sessionId: string): Promise<void> =>
  invoke(TauriCommand.MfaConfigCancel, { sessionId });

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
  // MFA configuration
  mfaConfigStart,
  mfaConfigSendCode,
  mfaConfigAuthorize,
  mfaConfigSetupStart,
  mfaConfigSetupFinish,
  mfaConfigSetupFido2,
  mfaConfigCancel,
};
