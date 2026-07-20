import { invoke } from '@tauri-apps/api/core';
import { generateWGKeys } from '../utils/generateWGKeys';
import { api } from './api';
import type {
  AddInstanceRequest,
  AddInstanceResult,
  CreateDeviceResponse,
  EnrollmentErrorKind,
  InstanceInfo,
  SaveDeviceConfigResponse,
  UpdateInstanceRequest,
  UpdateInstanceResult,
} from './types';
import { TauriCommand } from './types';

const getInstances = (): Promise<InstanceInfo[]> => invoke(TauriCommand.AllInstances);
const updateInstanceRecord = (args: {
  instanceId: number;
  response: CreateDeviceResponse;
}): Promise<void> => invoke(TauriCommand.UpdateInstance, args);
const saveDeviceConfig = (args: {
  privateKey: string;
  response: CreateDeviceResponse;
}): Promise<SaveDeviceConfigResponse> => invoke(TauriCommand.SaveDeviceConfig, args);

/** Extract the raw Rust error string from a Tauri command rejection. */
const rustErrorMessage = (err: unknown): string =>
  typeof err === 'object' && err !== null && 'message' in err
    ? String((err as Record<string, unknown>).message)
    : String(err);

/** Parse a Tauri command error that contains a serialized `EnrollmentError`
 *  JSON string into an `EnrollmentErrorKind` and human-readable message. */
export const parseEnrollmentError = (
  err: unknown,
): { error?: string; errorKind: EnrollmentErrorKind } => {
  const raw = rustErrorMessage(err);
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

/** True when a command error is a proxy 404 — the device was deleted
 *  server-side while a stale local record survived. */
const isDeviceNotFound = (err: unknown): boolean => {
  try {
    const parsed = JSON.parse(rustErrorMessage(err)) as {
      type?: string;
      status?: number;
    };
    return parsed.type === 'proxy_error' && parsed.status === 404;
  } catch {
    return false;
  }
};

export const enrollmentCreateDevice = async (
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

export const enrollmentAddInstance = async (
  values: AddInstanceRequest,
): Promise<AddInstanceResult> => {
  let sessionId: string | undefined;
  let handOffSession = false;
  try {
    const startResult = await api.enrollmentStart(values.url.trim(), values.token.trim());
    sessionId = startResult.session_id;

    const instances = await getInstances();
    const existing = instances.find((i) => i.uuid === startResult.instance.id);
    if (existing) {
      try {
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
        if (!isDeviceNotFound(e)) throw e;
        await api.deleteInstance(existing.id);
      }
    }

    const normalizedName = values.name.trim().toLowerCase();
    if (
      startResult.user.device_names.some((n) => n.trim().toLowerCase() === normalizedName)
    ) {
      return { error: `Device name '${values.name}' is already in use` };
    }

    handOffSession = true;
    return { startResponse: startResult, session_id: startResult.session_id };
  } catch (e) {
    const parsed = parseEnrollmentError(e);
    return { error: parsed.error, errorKind: parsed.errorKind };
  } finally {
    if (sessionId && !handOffSession) {
      await api.enrollmentFinish(sessionId).catch(() => {});
    }
  }
};

export const enrollmentUpdateInstance = async (
  values: UpdateInstanceRequest,
): Promise<UpdateInstanceResult> => {
  let sessionId: string | undefined;
  try {
    const instances = await getInstances();
    const existing = instances.find((i) => i.id === values.instanceId);
    if (!existing) return { error: 'Instance no longer exists.' };

    const startResult = await api.enrollmentStart(values.url, values.token);
    sessionId = startResult.session_id;

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
  } finally {
    if (sessionId) {
      await api.enrollmentFinish(sessionId).catch(() => {});
    }
  }
};
