const requireEnv = (name: string): string => {
	const value = process.env[name];
	if (!value) {
		throw new Error(`Missing required environment variable ${name}`);
	}
	return value;
};

const coreUrl = (): string => requireEnv("CORE_URL");
const proxyUrl = (): string => requireEnv("PROXY_URL");

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

export interface CoreNetwork {
	id: number;
	name: string;
	endpoint: string;
	port: number;
}

export type MfaMethod =
	| "totp"
	| "email"
	| "oidc"
	| "biometric"
	| "mobileapprove"
	| "fido2";

export interface MfaFlow {
	id: number;
	title: string;
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

	async listNetworks(): Promise<CoreNetwork[]> {
		return this.requestJson<CoreNetwork[]>("GET", "/api/v1/network");
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

	async createNetwork(data: unknown): Promise<CoreNetwork> {
		return this.requestJson<CoreNetwork>("POST", "/api/v1/network", data);
	}

	async updateNetwork(networkId: number, data: unknown): Promise<void> {
		await this.request("PUT", `/api/v1/network/${networkId}`, data);
	}

	async listGateways(networkId: number): Promise<Array<{ name: string }>> {
		return this.requestJson<Array<{ name: string }>>(
			"GET",
			`/api/v1/network/${networkId}/gateways`,
		);
	}

	async listMfaFlows(): Promise<MfaFlow[]> {
		return this.requestJson<MfaFlow[]>("GET", "/api/v1/mfa-flow");
	}

	async createMfaFlow(title: string, steps: MfaMethod[][]): Promise<MfaFlow> {
		return this.requestJson<MfaFlow>("POST", "/api/v1/mfa-flow", {
			title,
			steps: steps.map((methods) => ({ methods })),
		});
	}

	async deleteMfaFlow(flowId: number): Promise<void> {
		await this.request("DELETE", `/api/v1/mfa-flow/${flowId}`);
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

	async startDesktopConfiguration(username: string): Promise<string> {
		const data = await this.requestJson<{ enrollment_token: string }>(
			"POST",
			`/api/v1/user/${username}/start_desktop`,
			{
				send_enrollment_notification: false,
			},
		);
		return data.enrollment_token;
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
