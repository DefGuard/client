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

export type LocationMfaMode = "disabled" | "internal" | "external";

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

	// Core serves the web UI for any unrouted GET, so an endpoint the deployed core does not
	// have arrives as 200 text/html instead of a 404.
	private async requestJson<T>(
		method: string,
		apiPath: string,
		body?: unknown,
	): Promise<T> {
		const response = await this.request(method, apiPath, body);
		const contentType = response.headers.get("content-type") ?? "";
		if (!contentType.includes("application/json")) {
			throw new Error(
				`Core API ${method} ${apiPath} returned ${contentType || "no content type"} instead of JSON - the endpoint is missing from this core version`,
			);
		}
		return (await response.json()) as T;
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
		return this.requestJson<Array<{ id: number; name: string }>>(
			"GET",
			"/api/v1/network",
		);
	}

	async addUserDevice(name: string, pubkey: string): Promise<AddedUserDevice> {
		const username = process.env.CORE_ADMIN_USER ?? "admin";
		const data = await this.requestJson<{
			configs: DeviceConfig[];
			device: { id: number };
		}>("POST", `/api/v1/device/${username}`, {
			name,
			wireguard_pubkey: pubkey,
		});
		return { deviceId: data.device.id, configs: data.configs };
	}

	async deleteDevice(deviceId: number): Promise<void> {
		await this.request("DELETE", `/api/v1/device/${deviceId}`);
	}

	private async getNetworkDetails(
		networkId: number,
	): Promise<Record<string, unknown>> {
		return this.requestJson<Record<string, unknown>>(
			"GET",
			`/api/v1/network/${networkId}`,
		);
	}

	async setLocationMfaMode(
		networkId: number,
		mode: LocationMfaMode,
	): Promise<LocationMfaMode> {
		const current = await this.getNetworkDetails(networkId);
		const previous = current.location_mfa_mode as LocationMfaMode;
		if (previous === mode) {
			return previous;
		}
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
			peer_disconnect_threshold:
				mode === "disabled"
					? peerDisconnectThreshold
					: Math.max(
							peerDisconnectThreshold,
							MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA,
						),
			acl_enabled: current.acl_enabled === true,
			acl_default_allow: current.acl_default_allow === true,
			location_mfa_mode: mode,
			service_location_mode:
				typeof current.service_location_mode === "string"
					? current.service_location_mode
					: "disabled",
		});
		return previous;
	}

	// Core reports `mfa_required` during enrollment when any location on the instance enforces
	// internal MFA, so a test that expects no MFA has to clear every location, not just its own.
	async disableAllLocationMfa(): Promise<Map<number, LocationMfaMode>> {
		const previous = new Map<number, LocationMfaMode>();
		try {
			for (const network of await this.listNetworks()) {
				previous.set(
					network.id,
					await this.setLocationMfaMode(network.id, "disabled"),
				);
			}
		} catch (error) {
			await this.restoreLocationMfaModes(previous).catch(() => undefined);
			throw error;
		}
		return previous;
	}

	async restoreLocationMfaModes(
		modes: Map<number, LocationMfaMode>,
	): Promise<void> {
		for (const [networkId, mode] of modes) {
			await this.setLocationMfaMode(networkId, mode);
		}
	}

	private async startEnrollment(
		username: string,
		ephemeral: boolean,
	): Promise<EnrollmentFixture> {
		const data = await this.requestJson<{ enrollment_token: string }>(
			"POST",
			`/api/v1/user/${username}/start_enrollment`,
			{
				send_enrollment_notification: false,
			},
		);
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
