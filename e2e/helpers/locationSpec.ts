import type { CoreApi, CoreNetwork } from "./coreApi.js";

export const TEST_MFA_FLOW_PREFIX = "e2e-";
export const BASE_MFA_FLOW_TITLE = `${TEST_MFA_FLOW_PREFIX}totp`;

const requireEnv = (name: string): string => {
	const value = process.env[name];
	if (!value) {
		throw new Error(`Missing required environment variable ${name}`);
	}
	return value;
};

export const locationSpec = () => ({
	name: process.env.NETWORK_NAME ?? "e2e",
	address: process.env.NETWORK_ADDRESS ?? "10.10.10.1/24",
	endpoint: requireEnv("NETWORK_ENDPOINT"),
	port: Number(process.env.NETWORK_PORT ?? 51820),
	allowed_ips: process.env.NETWORK_ALLOWED_IPS ?? "10.10.10.0/24",
	dns: null as string | null,
	mtu: 1420,
	fwmark: 0,
	client_mtu: null as number | null,
	allow_all_groups: true,
	allowed_groups: [] as string[],
	keepalive_interval: 25,
	peer_disconnect_threshold: 300,
	acl_enabled: false,
	acl_default_allow: false,
	allowed_ips_from_acl: false,
	mfa_enabled: false,
	service_location_mode: "disabled",
	posture_checks: [] as number[],
	group_client_mtus: [] as unknown[],
});

export type LocationSpec = ReturnType<typeof locationSpec>;

const isTestNetwork = (network: CoreNetwork, spec: LocationSpec): boolean =>
	network.name === spec.name ||
	(network.endpoint === spec.endpoint && network.port === spec.port);

export const findTestNetwork = async (core: CoreApi): Promise<CoreNetwork> => {
	const spec = locationSpec();
	const networks = await core.listNetworks();
	if (networks.length !== 1) {
		throw new Error(
			`Core has ${networks.length} locations, the e2e suite expects exactly one ("${spec.name}")`,
		);
	}
	const [network] = networks;
	if (!isTestNetwork(network, spec)) {
		throw new Error(
			`Core location "${network.name}" is not the e2e location: expected name "${spec.name}" or endpoint ${spec.endpoint}:${spec.port}`,
		);
	}
	return network;
};

const ensureBaseMfaFlow = async (core: CoreApi): Promise<number> => {
	const existing = (await core.listMfaFlows()).find(
		(flow) => flow.title === BASE_MFA_FLOW_TITLE,
	);
	if (existing) {
		return existing.id;
	}
	return (await core.createMfaFlow(BASE_MFA_FLOW_TITLE, [["totp"]])).id;
};

const writeTestNetwork = async (
	core: CoreApi,
	networkId: number,
	changes: Partial<LocationSpec>,
): Promise<void> => {
	const baseFlowId = await ensureBaseMfaFlow(core);
	await core.updateNetwork(networkId, {
		...locationSpec(),
		...changes,
		mfa_flows: [{ flow_id: baseFlowId, is_default: true, group_ids: [] }],
	});
};

export const resetTestNetwork = async (core: CoreApi): Promise<number> => {
	const network = await findTestNetwork(core);
	await writeTestNetwork(core, network.id, {});
	for (const flow of await core.listMfaFlows()) {
		if (
			flow.title.startsWith(TEST_MFA_FLOW_PREFIX) &&
			flow.title !== BASE_MFA_FLOW_TITLE
		) {
			await core.deleteMfaFlow(flow.id);
		}
	}
	return network.id;
};

export const updateTestNetwork = async (
	core: CoreApi,
	changes: Partial<LocationSpec>,
): Promise<void> => {
	const network = await findTestNetwork(core);
	await writeTestNetwork(core, network.id, changes);
};
