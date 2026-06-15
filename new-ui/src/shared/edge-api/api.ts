import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import type {
  CreateDeviceResponse,
  InstanceInfo,
  MfaMethodValue,
  SaveDeviceConfigResponse,
} from '../rust-api/types';
import { TauriCommand } from '../rust-api/types';
import { generateWGKeys } from '../utils/generateWGKeys';
import { mfaToApi } from '../utils/mfa';
import type {
  ActivateUserRequest,
  ActivateUserResponse,
  AddInstanceRequest,
  AddInstanceResult,
  EdgeRequestHeaders,
  EnrollmentStartResponse,
  MfaSetupFinishRequest,
  MfaSetupFinishResponse,
  MfaSetupStartResponse,
} from './types';

const getPlatformHeader = (): Promise<string> => invoke(TauriCommand.GetPlatformHeader);
const getInstances = (): Promise<InstanceInfo[]> => invoke(TauriCommand.AllInstances);
const deleteInstance = (instanceId: number): Promise<void> =>
  invoke(TauriCommand.DeleteInstance, { instanceId });
const updateInstance = (args: {
  instanceId: number;
  response: CreateDeviceResponse;
}): Promise<void> => invoke(TauriCommand.UpdateInstance, args);
const saveDeviceConfig = (args: {
  privateKey: string;
  response: CreateDeviceResponse;
}): Promise<SaveDeviceConfigResponse> => invoke(TauriCommand.SaveDeviceConfig, args);

const buildProxyUrl = (url: string): string => {
  const base = url.endsWith('/') ? url.slice(0, -1) : url;
  return `${base}/api/v1`;
};

const getEdgeRequestHeaders = async (): Promise<EdgeRequestHeaders> => {
  const platform = await getPlatformHeader();
  const version = await getVersion().catch(() => 'unknown');
  return {
    'defguard-client-platform': platform,
    'defguard-client-version': version,
  };
};

const createDevice = async (
  proxyUrl: string,
  cookie: string,
  name: string,
): Promise<{ error?: string }> => {
  const edgeHeaders = await getEdgeRequestHeaders();
  const { publicKey, privateKey } = generateWGKeys();
  const url = buildProxyUrl(proxyUrl);
  const deviceRes = await fetch(`${url}/enrollment/create_device`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Cookie: cookie,
      ...edgeHeaders,
    },
    body: JSON.stringify({ name, pubkey: publicKey }),
  });

  if (!deviceRes.ok) {
    const body = (await deviceRes.json()) as { error?: string };
    return { error: body.error ?? `create_device failed (${deviceRes.status})` };
  }

  await saveDeviceConfig({ privateKey, response: await deviceRes.json() });
  return {};
};

const addInstance = async (values: AddInstanceRequest): Promise<AddInstanceResult> => {
  try {
    const proxyUrl = buildProxyUrl(values.url);

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
      // device no longer exists core side, clean it up
      if (netRes.status === 404) {
        await deleteInstance(existing.id);
      } else {
        if (!netRes.ok) return { error: `network_info failed (${netRes.status})` };
        await updateInstance({ instanceId: existing.id, response: await netRes.json() });
        return {};
      }
    }

    const normalizedName = values.name.trim().toLowerCase();
    if (resp.user.device_names.some((n) => n.trim().toLowerCase() === normalizedName)) {
      return { error: `Device name '${values.name}' is already in use` };
    }

    return { startResponse: resp, proxyUrl, cookie };
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
    const base = buildProxyUrl(proxyUrl);
    const edgeHeaders = await getEdgeRequestHeaders();
    const res = await fetch(`${base}/enrollment/register-mfa/code/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Cookie: cookie, ...edgeHeaders },
      body: JSON.stringify({ method: mfaToApi(method) }),
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
    const base = buildProxyUrl(proxyUrl);
    const edgeHeaders = await getEdgeRequestHeaders();
    const res = await fetch(`${base}/enrollment/activate_user`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Cookie: cookie, ...edgeHeaders },
      body: JSON.stringify({ ...request }),
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
    const base = buildProxyUrl(proxyUrl);
    const edgeHeaders = await getEdgeRequestHeaders();
    const res = await fetch(`${base}/enrollment/register-mfa/code/finish`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Cookie: cookie, ...edgeHeaders },
      body: JSON.stringify({
        code: request.code,
        method: mfaToApi(request.method),
      }),
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

export const edgeApi = {
  getEdgeRequestHeaders,
  createDevice,
  addInstance,
  startMfaSetup,
  activateUser,
  finishMfaSetup,
};
