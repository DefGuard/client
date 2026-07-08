const MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA = 120;

const requireEnv = (name: string): string => {
  const value = process.env[name];
  if (!value) {
    throw new Error(`Missing required environment variable ${name}`);
  }
  return value;
};

const coreUrl = (): string => requireEnv('CORE_URL');
const proxyUrl = (): string => requireEnv('PROXY_URL');

export type LocationMfaMode = 'disabled' | 'internal' | 'external';

export interface EnrollmentFixture {
  username: string;
  enrollmentToken: string;
  enrollmentUrl: string;
  ephemeral: boolean;
}

export class CoreApi {
  private cookie = '';

  private async request(
    method: string,
    apiPath: string,
    body?: unknown,
  ): Promise<Response> {
    const response = await fetch(`${coreUrl()}${apiPath}`, {
      method,
      redirect: 'manual',
      headers: {
        'Content-Type': 'application/json',
        ...(this.cookie ? { Cookie: this.cookie } : {}),
      },
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
    if (response.status >= 300 && response.status < 400) {
      throw new Error(`Core API ${method} ${apiPath} redirected — check CORE_URL`);
    }
    if (!response.ok) {
      throw new Error(
        `Core API ${method} ${apiPath} failed: ${response.status} ${await response.text()}`,
      );
    }
    return response;
  }

  async login(): Promise<void> {
    const response = await this.request('POST', '/api/v1/auth', {
      username: process.env.CORE_ADMIN_USER ?? 'admin',
      password: requireEnv('CORE_ADMIN_PASSWORD'),
    });
    const setCookie = response.headers.get('set-cookie');
    if (!setCookie) {
      throw new Error('Core API login did not return a session cookie');
    }
    this.cookie = setCookie.split(';')[0];
  }

  async userExists(username: string): Promise<boolean> {
    const response = await fetch(`${coreUrl()}/api/v1/user/${username}`, {
      redirect: 'manual',
      headers: this.cookie ? { Cookie: this.cookie } : {},
    });
    return response.ok;
  }

  async createUser(username: string): Promise<void> {
    await this.request('POST', '/api/v1/user', {
      username,
      first_name: 'E2E',
      last_name: 'Test',
      email: `${username}@e2e.test`,
    });
  }

  async deleteUser(username: string): Promise<void> {
    await this.request('DELETE', `/api/v1/user/${username}`);
  }

  async listNetworks(): Promise<
    Array<{ id: number; location_mfa_mode: LocationMfaMode }>
  > {
    const response = await this.request('GET', '/api/v1/network');
    return (await response.json()) as Array<{
      id: number;
      location_mfa_mode: LocationMfaMode;
    }>;
  }

  async findAvailableIp(networkId: number): Promise<string> {
    const response = await this.request('GET', `/api/v1/device/network/ip/${networkId}`);
    const ips = (await response.json()) as Array<{ ip: string }>;
    return ips[0].ip;
  }

  async addNetworkDevice(
    networkId: number,
    name: string,
    assignedIps: string[],
    pubkey: string,
  ): Promise<string> {
    const response = await this.request('POST', '/api/v1/device/network', {
      name,
      description: null,
      location_id: networkId,
      assigned_ips: assignedIps,
      wireguard_pubkey: pubkey,
    });
    const data = (await response.json()) as { config: { config: string } };
    return data.config.config;
  }

  async setLocationMfaMode(
    networkId: number,
    mode: LocationMfaMode,
  ): Promise<LocationMfaMode> {
    const current = (await (
      await this.request('GET', `/api/v1/network/${networkId}`)
    ).json()) as Record<string, unknown>;
    const previous = current.location_mfa_mode as LocationMfaMode;
    if (previous === mode) {
      return previous;
    }
    const joinList = (value: unknown): string =>
      Array.isArray(value) ? value.join(',') : ((value as string | null) ?? '');
    await this.request('PUT', `/api/v1/network/${networkId}`, {
      name: current.name,
      address: joinList(current.address),
      endpoint: current.endpoint,
      port: current.port,
      allowed_ips: joinList(current.allowed_ips) || null,
      dns: (current.dns as string | null) ?? null,
      mtu: current.mtu,
      fwmark: current.fwmark,
      allow_all_groups: current.allow_all_groups,
      allowed_groups: current.allowed_groups ?? [],
      keepalive_interval: current.keepalive_interval,
      peer_disconnect_threshold: Math.max(
        Number(current.peer_disconnect_threshold ?? 0),
        MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA,
      ),
      acl_enabled: current.acl_enabled,
      acl_default_allow: current.acl_default_allow,
      location_mfa_mode: mode,
      service_location_mode: current.service_location_mode ?? 'disabled',
    });
    return previous;
  }

  private async startEnrollment(
    username: string,
    ephemeral: boolean,
  ): Promise<EnrollmentFixture> {
    const response = await this.request(
      'POST',
      `/api/v1/user/${username}/start_enrollment`,
      {
        send_enrollment_notification: false,
      },
    );
    const data = (await response.json()) as { enrollment_token: string };
    return {
      username,
      enrollmentToken: data.enrollment_token,
      enrollmentUrl: proxyUrl(),
      ephemeral,
    };
  }

  // A user with a pending enrollment, always (re)created so it has not enrolled.
  async createEnrollmentFixture(): Promise<EnrollmentFixture> {
    const pinned = process.env.TEST_USERNAME;
    const username = pinned ?? `e2e${Math.floor(Math.random() * 1_000_000)}`;
    if (await this.userExists(username)) {
      await this.deleteUser(username);
    }
    await this.createUser(username);
    return this.startEnrollment(username, !pinned);
  }
}

export const loggedInCoreApi = async (): Promise<CoreApi> => {
  const api = new CoreApi();
  await api.login();
  return api;
};
