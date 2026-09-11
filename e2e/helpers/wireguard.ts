import { generateKeyPairSync } from "node:crypto";
import type { CoreApi } from "./coreApi.js";

export const generateWireguardKeys = () => {
	const { privateKey, publicKey } = generateKeyPairSync("x25519");
	return {
		privateKey: privateKey
			.export({ type: "pkcs8", format: "der" })
			.subarray(-32)
			.toString("base64"),
		publicKey: publicKey
			.export({ type: "spki", format: "der" })
			.subarray(-32)
			.toString("base64"),
	};
};

export type TunnelConfig = {
	name: string;
	deviceId: number;
	prvkey: string;
	pubkey: string;
	address: string;
	serverPubkey: string;
	allowedIps: string;
	endpoint: string;
	dns: string;
	keepalive: string;
};

export const provisionTunnel = async (
	core: CoreApi,
	networkId: number,
	name: string,
): Promise<TunnelConfig> => {
	const keys = generateWireguardKeys();
	const { deviceId, configs } = await core.addUserDevice(name, keys.publicKey);
	const config = configs.find((item) => item.network_id === networkId);
	if (!config) {
		throw new Error(`Device ${name} was not added to location ${networkId}`);
	}
	return {
		name,
		deviceId,
		prvkey: keys.privateKey,
		pubkey: keys.publicKey,
		address: config.address.join(","),
		serverPubkey: config.pubkey,
		allowedIps: config.allowed_ips.join(","),
		endpoint: config.endpoint,
		dns: config.dns ?? "",
		keepalive: String(config.keepalive_interval),
	};
};
