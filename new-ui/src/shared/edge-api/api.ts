import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import { api } from '../rust-api/api';
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
  EnrollmentErrorKind,
  MfaSetupFinishRequest,
  MfaSetupFinishResponse,
  MfaSetupStartResponse,
  UpdateInstanceRequest,
  UpdateInstanceResult,
} from './types';

const getInstances = (): Promise<InstanceInfo[]> => invoke(TauriCommand.AllInstances);
const updateInstanceRecord = (args: {
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
  const platform = (await invoke(TauriCommand.GetPlatformHeader)) as string;
  const version = await getVersion().catch(() => 'unknown');
  return {
    'defguard-client-platform': platform,
    'defguard-client-version': version,
  };
};

/// Parse a Tauri command error that contains a serialized `EnrollmentError`
/// JSON string into an `EnrollmentErrorKind` and human-readable message.
const parseEnrollmentError = (
  err: unknown,
): { error?: string; errorKind: EnrollmentErrorKind } => {
  // Tauri 2 command errors are plain objects, not Error instances.
  // The Rust error message is on the `message` property.
  const raw =
    typeof err === 'object' && err !== null && 'message' in err
      ? String((err as Record<string, unknown>).message)
      : String(err);
  try {
    const parsed = JSON.parse(raw) as { type: string; message?: string; status?: number };
    switch (parsed.type) {
      case 'token_expired':
        return { errorKind: 'unauthorized' };
      case 'network_error':
        return { errorKind: 'network' };
      case 'proxy_error':
        return { error: parsed.message, errorKind: 'server' };
      default:
        return { error: parsed.message ?? raw, errorKind: 'server' };
    }
  } catch {
    return { error: raw, errorKind: 'server' };
  }
};

const createDevice = async (
  sessionId: string,
  name: string,
): Promise<{ error?: string }> => {
  try {
    const { publicKey, privateKey } = generateWGKeys();
    const deviceResponse = await api.enrollmentCreateDevice(sessionId, name, publicKey);
    await saveDeviceConfig({
      privateKey,
      response: deviceResponse as CreateDeviceResponse,
    });
    return {};
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
};

const addInstance = async (values: AddInstanceRequest): Promise<AddInstanceResult> => {
  try {
    const startResult = await api.enrollmentStart(values.url.trim(), values.token.trim());

    const instances = await getInstances();
    const existing = instances.find((i) => i.uuid === startResult.instance.id);
    if (existing) {
      const netInfo = await api.enrollmentNetworkInfo(
        startResult.session_id,
        existing.pubkey,
      );
      await updateInstanceRecord({
        instanceId: existing.id,
        response: netInfo as CreateDeviceResponse,
      });
      return {};
    }

    const normalizedName = values.name.trim().toLowerCase();
    if (
      startResult.user.device_names.some((n) => n.trim().toLowerCase() === normalizedName)
    ) {
      return { error: `Device name '${values.name}' is already in use` };
    }

    return { startResponse: startResult, session_id: startResult.session_id };
  } catch (e) {
    const parsed = parseEnrollmentError(e);
    return { error: parsed.error, errorKind: parsed.errorKind };
  }
};

const updateExistingInstance = async (
  values: UpdateInstanceRequest,
): Promise<UpdateInstanceResult> => {
  try {
    const instances = await getInstances();
    const existing = instances.find((i) => i.id === values.instanceId);
    if (!existing) return { error: 'Instance no longer exists.' };

    const startResult = await api.enrollmentStart(values.url, values.token);

    if (startResult.instance.id !== existing.uuid) {
      return {
        error: 'Provided token belongs to a different instance.',
        errorKind: 'unauthorized',
      };
    }

    const netInfo = await api.enrollmentNetworkInfo(
      startResult.session_id,
      existing.pubkey,
    );
    await updateInstanceRecord({
      instanceId: existing.id,
      response: netInfo as CreateDeviceResponse,
    });
    return {};
  } catch (e) {
    const parsed = parseEnrollmentError(e);
    return { error: parsed.error, errorKind: parsed.errorKind };
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
  updateInstance: updateExistingInstance,
  startMfaSetup,
  activateUser,
  finishMfaSetup,
};
