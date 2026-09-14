const MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA = 120;

const requireEnv = (name: string): string => {
	const value = process.env[name];
	if (!value) {
		throw new Error(`Missing required environment variable ${name}`);
	}
	return value;
};

const coreUrl = (): string => requireEnv("CORE_URL");
const proxyUrl = (): string => requireEnv("PROXY_URL");

export interface LocationMfaFlowAssignment {
	flow_id: number;
	is_default: boolean;
	group_ids: number[];
}

export interface LocationMfaState {
	mfaEnabled: boolean;
	mfaFlows: LocationMfaFlowAssignment[];
}

export interface DeviceConfig {
	network_id: number;
	network_name: string;
	config: string;
	address: string[];
	endpoint: string;
	allowed_ips: string[];
	pubkey: string;
	dns: string | null;
	keepalive_interval: number;
}

export interface AddedUserDevice {
	deviceId: number;
	configs: DeviceConfig[];
}

export interface EnrollmentFixture {
	username: string;
	enrollmentToken: string;
	enrollmentUrl: string;
	ephemeral: boolean;
}

export class CoreApi {
	private cookie = "";

	private async request(
		method: string,
		apiPath: string,
		body?: unknown,
	): Promise<Response> {
		const response = await fetch(`${coreUrl()}${apiPath}`, {
			method,
			redirect: "manual",
			headers: {
				"Content-Type": "application/json",
				...(this.cookie ? { Cookie: this.cookie } : {}),
			},
			body: body !== undefined ? JSON.stringify(body) : undefined,
		});
		if (response.status >= 300 && response.status < 400) {
			throw new Error(
				`Core API ${method} ${apiPath} redirected — check CORE_URL`,
			);
		}
		if (!response.ok) {
			throw new Error(
				`Core API ${method} ${apiPath} failed: ${response.status} ${await response.text()}`,
			);
		}
		return response;
	}

	async login(): Promise<void> {
		const response = await this.request("POST", "/api/v1/auth", {
			username: process.env.CORE_ADMIN_USER ?? "admin",
			password: requireEnv("CORE_ADMIN_PASSWORD"),
		});
		const setCookie = response.headers.get("set-cookie");
		if (!setCookie) {
			throw new Error("Core API login did not return a session cookie");
		}
		this.cookie = setCookie.split(";")[0];
	}

	async userExists(username: string): Promise<boolean> {
		const response = await fetch(`${coreUrl()}/api/v1/user/${username}`, {
			redirect: "manual",
			headers: this.cookie ? { Cookie: this.cookie } : {},
		});
		return response.ok;
	}

	async createUser(username: string): Promise<void> {
		await this.request("POST", "/api/v1/user", {
			username,
			first_name: "E2E",
			last_name: "Test",
			email: `${username}@e2e.test`,
		});
	}

	async deleteUser(username: string): Promise<void> {
		await this.request("DELETE", `/api/v1/user/${username}`);
	}

	async listNetworks(): Promise<Array<{ id: number; name: string }>> {
		const response = await this.request("GET", "/api/v1/network");
		return (await response.json()) as Array<{ id: number; name: string }>;
	}

	async addUserDevice(name: string, pubkey: string): Promise<AddedUserDevice> {
		const username = process.env.CORE_ADMIN_USER ?? "admin";
		const response = await this.request("POST", `/api/v1/device/${username}`, {
			name,
			wireguard_pubkey: pubkey,
		});
		const data = (await response.json()) as {
			configs: DeviceConfig[];
			device: { id: number };
		};
		return { deviceId: data.device.id, configs: data.configs };
	}

	async deleteDevice(deviceId: number): Promise<void> {
		await this.request("DELETE", `/api/v1/device/${deviceId}`);
	}

	private async getNetworkDetails(
		networkId: number,
	): Promise<Record<string, unknown>> {
		const response = await this.request("GET", `/api/v1/network/${networkId}`);
		return (await response.json()) as Record<string, unknown>;
	}

	async getLocationMfaState(networkId: number): Promise<LocationMfaState> {
		const current = await this.getNetworkDetails(networkId);
		const response = await this.request(
			"GET",
			`/api/v1/location/${networkId}/mfa-flows`,
		);
		const flows = (await response.json()) as Array<{
			id: number;
			is_default: boolean;
			groups: Array<{ id: number }>;
		}>;
		return {
			mfaEnabled: current.mfa_enabled === true,
			mfaFlows: flows.map((flow) => ({
				flow_id: flow.id,
				is_default: flow.is_default,
				group_ids: flow.groups.map((group) => group.id),
			})),
		};
	}

	private async updateLocationMfaState(
		networkId: number,
		state: LocationMfaState,
	): Promise<void> {
		const current = await this.getNetworkDetails(networkId);
		const joinList = (value: unknown): string =>
			Array.isArray(value)
				? value.join(",")
				: typeof value === "string"
					? value
					: "";
		const peerDisconnectThreshold = Number(
			current.peer_disconnect_threshold ?? 0,
		);
		await this.request("PUT", `/api/v1/network/${networkId}`, {
			name: current.name,
			address: joinList(current.address),
			endpoint: current.endpoint,
			port: current.port,
			allowed_ips: joinList(current.allowed_ips) || null,
			dns: typeof current.dns === "string" ? current.dns : null,
			mtu: current.mtu,
			fwmark: current.fwmark,
			allow_all_groups: current.allow_all_groups === true,
			allowed_groups: Array.isArray(current.allowed_groups)
				? current.allowed_groups
				: [],
			keepalive_interval: current.keepalive_interval,
			peer_disconnect_threshold: state.mfaEnabled
				? Math.max(
						peerDisconnectThreshold,
						MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA,
					)
				: peerDisconnectThreshold,
			acl_enabled: current.acl_enabled === true,
			acl_default_allow: current.acl_default_allow === true,
			allowed_ips_from_acl: current.allowed_ips_from_acl === true,
			mfa_enabled: state.mfaEnabled,
			service_location_mode:
				typeof current.service_location_mode === "string"
					? current.service_location_mode
					: "disabled",
			posture_checks: Array.isArray(current.posture_checks)
				? current.posture_checks
				: [],
			mfa_flows: state.mfaFlows,
		});
	}

	async setLocationMfaState(
		networkId: number,
		state: LocationMfaState,
	): Promise<LocationMfaState> {
		const previous = await this.getLocationMfaState(networkId);
		await this.updateLocationMfaState(networkId, state);
		return previous;
	}

	async setLocationMfaEnabled(
		networkId: number,
		enabled: boolean,
	): Promise<LocationMfaState> {
		const previous = await this.getLocationMfaState(networkId);
		await this.updateLocationMfaState(networkId, {
			...previous,
			mfaEnabled: enabled,
		});
		return previous;
	}

	async disableAllLocationMfa(): Promise<Map<number, LocationMfaState>> {
		const previous = new Map<number, LocationMfaState>();
		try {
			for (const network of await this.listNetworks()) {
				previous.set(
					network.id,
					await this.setLocationMfaEnabled(network.id, false),
				);
			}
		} catch (error) {
			await this.restoreLocationMfaStates(previous).catch(() => undefined);
			throw error;
		}
		return previous;
	}

	async restoreLocationMfaStates(
		states: Map<number, LocationMfaState>,
	): Promise<void> {
		for (const [networkId, state] of states) {
			await this.setLocationMfaState(networkId, state);
		}
	}

	async createTotpFlow(): Promise<number> {
		const response = await this.request("POST", "/api/v1/mfa-flow", {
			title: `e2e-totp-${Date.now()}`,
			steps: [{ methods: ["totp"] }],
		});
		const data = (await response.json()) as { id: number };
		return data.id;
	}

	async deleteMfaFlow(flowId: number): Promise<void> {
		await this.request("DELETE", `/api/v1/mfa-flow/${flowId}`);
	}

	private async startEnrollment(
		username: string,
		ephemeral: boolean,
	): Promise<EnrollmentFixture> {
		const response = await this.request(
			"POST",
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
