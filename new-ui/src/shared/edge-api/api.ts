import { invoke } from '@tauri-apps/api/core';
import { api } from '../rust-api/api';
import type {
  CreateDeviceResponse,
  InstanceInfo,
  SaveDeviceConfigResponse,
} from '../rust-api/types';
import { TauriCommand } from '../rust-api/types';
import { generateWGKeys } from '../utils/generateWGKeys';
import type {
  AddInstanceRequest,
  AddInstanceResult,
  EnrollmentErrorKind,
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
  let sessionId: string | undefined;
  // The successful new-enrollment path hands the session to the enrollment
  // wizard, which owns its lifecycle from there. Every other exit (existing
  // instance, name clash, error) must release the session it created.
  let handOffSession = false;
  try {
    const startResult = await api.enrollmentStart(values.url.trim(), values.token.trim());
    sessionId = startResult.session_id;

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

    handOffSession = true;
    return { startResponse: startResult, session_id: startResult.session_id };
  } catch (e) {
    const parsed = parseEnrollmentError(e);
    return { error: parsed.error, errorKind: parsed.errorKind };
  } finally {
    // Best-effort cleanup; enrollment_finish errors if the session is already
    // gone, so swallow failures rather than mask the real result.
    if (sessionId && !handOffSession) {
      await api.enrollmentFinish(sessionId).catch(() => {});
    }
  }
};

const updateExistingInstance = async (
  values: UpdateInstanceRequest,
): Promise<UpdateInstanceResult> => {
  // The update flow only needs the session for the network_info call; it never
  // hands it to the wizard, so every exit after start() must release it.
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
    // Best-effort cleanup; guarded so we never finish a session that was never
    // created (e.g. the instance-missing early return, or a start() failure).
    if (sessionId) {
      await api.enrollmentFinish(sessionId).catch(() => {});
    }
  }
};

export const edgeApi = {
  createDevice,
  addInstance,
  updateInstance: updateExistingInstance,
};
