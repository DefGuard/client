import { getVersion } from '@tauri-apps/api/app';

import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import { generateWGKeys } from '../utils/generateWGKeys';
import { enrollmentToMfaMethod } from '../utils/mfa';
import type {
  ActivateUserRequest,
  ActivateUserResponse,
  ActiveConnectionSummary,
  AddInstanceRequest,
  AddInstanceResult,
  AppConfig,
  AppConfigPatch,
  Connection,
  ConnectionArgs,
  EdgeRequestHeaders,
  EnrollmentStartResponse,
  InstanceInfo,
  LocationDetails,
  LocationDetailsArgs,
  LocationInfo,
  LocationStats,
  MfaMethodValue,
  MfaSetupFinishRequest,
  MfaSetupFinishResponse,
  MfaSetupStartResponse,
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

const getPostureData = async (): Promise<unknown> => invoke(TauriCommand.GetPostureData);

const swapToFullView = async () => invoke(TauriCommand.SwapToFullView);

const swapToTray = async () => invoke(TauriCommand.SwapToTray);

const closeTrayWindow = async () => invoke(TauriCommand.CloseTrayWindow);

const addInstance = async (values: AddInstanceRequest): Promise<AddInstanceResult> => {
  try {
    let proxyUrl = values.url;
    if (proxyUrl.endsWith('/')) proxyUrl = proxyUrl.slice(0, -1);
    proxyUrl = `${proxyUrl}/api/v1`;

    const edgeHeaders = await getEdgeRequestHeaders();

    const startRes = await fetch(`${proxyUrl}/enrollment/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...edgeHeaders },
      body: JSON.stringify({ token: values.token }),
    });

    if (!startRes.ok) {
      const body = (await startRes.json()) as { error?: string };
      return { error: body.error ?? `Enrollment start failed (${startRes.status})` };
    }

    const cookie = startRes.headers
      .getSetCookie()
      .find((c) => c.startsWith('defguard_proxy='));
    if (!cookie) return { error: 'Auth cookie missing from enrollment response' };

    const resp = (await startRes.json()) as EnrollmentStartResponse;
    console.log({ resp });

    const instances = await getInstances();
    const existing = instances.find((i) => i.uuid === resp.instance.id);
    if (existing) {
      const netRes = await fetch(`${proxyUrl}/enrollment/network_info`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Cookie: cookie,
          ...edgeHeaders,
        },
        body: JSON.stringify({ pubkey: existing.pubkey }),
      });
      if (!netRes.ok) return { error: `network_info failed (${netRes.status})` };
      await updateInstance({ instanceId: existing.id, response: await netRes.json() });
      return {};
    }

    const { publicKey, privateKey } = generateWGKeys();
    const deviceRes = await fetch(`${proxyUrl}/enrollment/create_device`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
        ...edgeHeaders,
      },
      body: JSON.stringify({ name: values.name, pubkey: publicKey }),
    });

    if (!deviceRes.ok) {
      const body = (await deviceRes.json()) as { error?: string };
      return { error: body.error ?? `create_device failed (${deviceRes.status})` };
    }

    await saveDeviceConfig({ privateKey, response: await deviceRes.json() });

    // Show enrollment
    if (!resp.user.enrolled) {
      return { startResponse: resp, proxyUrl, cookie };
    }

    return {};
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
};

const startMfaSetup = async (
  proxyUrl: string,
  cookie: string,
  method: MfaMethodValue,
): Promise<{ result?: MfaSetupStartResponse; error?: string }> => {
  try {
    const edgeHeaders = await getEdgeRequestHeaders();
    const res = await fetch(`${proxyUrl}/enrollment/register-mfa/code/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Cookie: cookie, ...edgeHeaders },
      body: JSON.stringify({ method: enrollmentToMfaMethod(method) }),
    });
    if (!res.ok) {
      const body = (await res.json()) as { error?: string };
      return { error: body.error ?? `MFA setup start failed (${res.status})` };
    }
    return { result: (await res.json()) as MfaSetupStartResponse };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
};

const activateUser = async (
  proxyUrl: string,
  cookie: string,
  request: Omit<ActivateUserRequest, 'phone_number'>,
): Promise<{ result?: ActivateUserResponse; error?: string }> => {
  try {
    const edgeHeaders = await getEdgeRequestHeaders();
    const res = await fetch(`${proxyUrl}/enrollment/activate_user`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Cookie: cookie, ...edgeHeaders },
      body: JSON.stringify({ ...request, phone_number: '' }),
    });
    if (!res.ok) {
      const body = (await res.json()) as { error?: string };
      return { error: body.error ?? `activate_user failed (${res.status})` };
    }
    return { result: (await res.json()) as ActivateUserResponse };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
};

const finishMfaSetup = async (
  proxyUrl: string,
  cookie: string,
  request: MfaSetupFinishRequest,
): Promise<{ result?: MfaSetupFinishResponse; error?: string }> => {
  try {
    const edgeHeaders = await getEdgeRequestHeaders();
    const res = await fetch(`${proxyUrl}/enrollment/register-mfa/code/finish`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Cookie: cookie, ...edgeHeaders },
      body: JSON.stringify(request),
    });
    if (!res.ok) {
      const body = (await res.json()) as { error?: string };
      return { error: body.error ?? `MFA setup finish failed (${res.status})` };
    }
    return { result: (await res.json()) as MfaSetupFinishResponse };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
};

export const api = {
  getEdgeRequestHeaders,
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
  // Enrollment
  addInstance,
  activateUser,
  startMfaSetup,
  finishMfaSetup,
};
