import { fetch } from '@tauri-apps/plugin-http';
import { api } from '../../../rust-api/api';
import type {
  EdgeRequestHeaders,
  InstanceInfo,
  LocationInfo,
} from '../../../rust-api/types';

export const CLIENT_MFA_ENDPOINT = 'api/v1/client-mfa';

export class MfaStartError extends Error {
  public readonly status?: number;

  constructor(message: string, status?: number) {
    super(message);
    this.name = 'MfaStartError';
    this.status = status;
  }
}

export type MfaStartMethod = 0 | 1 | 2 | 4;

export type MfaStartResponse = {
  token: string;
  challenge?: string;
};

type MfaStartErrorResponse = {
  error?: string;
};

export const shouldShowPostureError = (
  err: unknown,
  location: LocationInfo,
): err is MfaStartError =>
  err instanceof MfaStartError && err.status === 403 && location.posture_check_required;

type StartClientMfaSessionParams = {
  instance: InstanceInfo;
  location: LocationInfo;
  method: MfaStartMethod;
};

type StartClientMfaSessionResult = {
  response: MfaStartResponse;
  headers: EdgeRequestHeaders;
};

export const startClientMfaSession = async ({
  instance,
  location,
  method,
}: StartClientMfaSessionParams): Promise<StartClientMfaSessionResult> => {
  let headers: EdgeRequestHeaders;
  try {
    headers = await api.getEdgeRequestHeaders();
  } catch {
    throw new MfaStartError('Failed to load request headers');
  }

  let posture_data: unknown;
  try {
    posture_data = location.posture_check_required
      ? await api.getPostureData()
      : undefined;
  } catch {
    throw new MfaStartError('Failed to load posture data');
  }

  try {
    const response = await fetch(`${instance.proxy_url}${CLIENT_MFA_ENDPOINT}/start`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...headers,
      },
      body: JSON.stringify({
        method,
        pubkey: instance.pubkey,
        location_id: location.network_id,
        posture_data,
      }),
    });

    if (!response.ok) {
      let message = 'Failed to start MFA';
      try {
        const data = (await response.json()) as MfaStartErrorResponse;
        message = data.error ?? message;
      } catch {
        // Keep the response status even if the proxy sends a malformed error body.
      }
      throw new MfaStartError(message, response.status);
    }

    return {
      response: (await response.json()) as MfaStartResponse,
      headers,
    };
  } catch (err) {
    if (err instanceof MfaStartError) {
      throw err;
    }
    throw new MfaStartError('Failed to reach server');
  }
};
