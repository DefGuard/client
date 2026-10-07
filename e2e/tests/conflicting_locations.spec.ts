import { $, browser, expect } from "@wdio/globals";
import { invoke } from "../helpers/client.js";
import {
	connectAndPing,
	disconnect,
	FULL_MFA_VIEW,
	waitForGatewayPing,
} from "../helpers/connection.js";
import {
	type CoreApi,
	type LocationMfaMode,
	loggedInCoreApi,
} from "../helpers/coreApi.js";
import {
	selectTunnel,
	switchToFullView,
	switchToTrayView,
	switchToWindowLabel,
} from "../helpers/windows.js";
import {
	generateWireguardKeys,
	provisionTunnel,
	type TunnelConfig,
} from "../helpers/wireguard.js";

type TunnelEntry = { id: number; name: string; active: boolean };

const saveTunnel = (config: Omit<TunnelConfig, "deviceId">) =>
	invoke("save_tunnel", {
		tunnel: {
			name: config.name,
			pubkey: config.pubkey,
			prvkey: config.prvkey,
			address: config.address,
			server_pubkey: config.serverPubkey,
			preshared_key: "",
			allowed_ips: config.allowedIps,
			endpoint: config.endpoint,
			dns: "",
			persistent_keep_alive: Number(config.keepalive),
			route_all_traffic: false,
			pre_up: "",
			post_up: "",
			pre_down: "",
			post_down: "",
		},
	});

const listTunnels = () => invoke<TunnelEntry[]>("all_tunnels");

const waitForActiveTunnels = (names: string[]) =>
	browser.waitUntil(
		async () => {
			const active = (await listTunnels())
				.filter((tunnel) => tunnel.active)
				.map((tunnel) => tunnel.name)
				.sort();
			return JSON.stringify(active) === JSON.stringify([...names].sort());
		},
		{ timeoutMsg: `Expected active tunnels: ${names.join(", ")}` },
	);

const clickConnect = async () => {
	const button = $(".connect-button.disconnected");
	await button.waitForClickable();
	await button.click();
};

const conflictModal = () => $("#confirm-modal");

const expectConflictModal = async (conflictName: string) => {
	await conflictModal().waitForDisplayed();
	await expect(conflictModal()).toHaveText("Connection unavailable", {
		containing: true,
	});
	await expect(conflictModal().$$("li")).toBeElementsArrayOfSize(1);
	await expect(conflictModal().$("li")).toHaveText(conflictName);
};

const closeConflictModal = async (buttonText: string) => {
	const button = conflictModal().$(`button=${buttonText}`);
	await button.waitForClickable();
	await button.click();
	await conflictModal().waitForDisplayed({ reverse: true });
};

describe("conflicting locations", () => {
	let core: CoreApi;
	let networkId: number;
	let previousMfaMode: LocationMfaMode | undefined;
	let first: TunnelConfig;
	let second: TunnelConfig;
	let split: Omit<TunnelConfig, "deviceId">;

	before(async () => {
		core = await loggedInCoreApi();
		const networkName = process.env.NETWORK_NAME ?? "e2e";
		const network = (await core.listNetworks()).find(
			(item) => item.name === networkName,
		);
		if (!network) {
			throw new Error(`Core network "${networkName}" was not found`);
		}
		networkId = network.id;
		previousMfaMode = await core.setLocationMfaMode(networkId, "disabled");

		const stamp = Date.now();
		first = await provisionTunnel(core, networkId, `e2e-conflict-a-${stamp}`);
		second = await provisionTunnel(core, networkId, `e2e-conflict-b-${stamp}`);
		const keys = generateWireguardKeys();
		// Documentation ranges (RFC 5737) never overlap the test network or the host LAN.
		split = {
			...first,
			name: `e2e-conflict-split-${stamp}`,
			prvkey: keys.privateKey,
			pubkey: keys.publicKey,
			serverPubkey: generateWireguardKeys().publicKey,
			address: "198.51.100.2/32",
			allowedIps: "203.0.113.0/24",
		};

		await switchToFullView();
		for (const tunnel of [first, second, split]) {
			await saveTunnel(tunnel);
		}
		await browser.refresh();
	});

	after(async () => {
		if (previousMfaMode) {
			await core.setLocationMfaMode(networkId, previousMfaMode);
		}
		await switchToFullView();
		for (const tunnel of await listTunnels()) {
			await invoke("delete_tunnel", { tunnelId: tunnel.id });
		}
		for (const config of [first, second]) {
			if (config) {
				await core.deleteDevice(config.deviceId);
			}
		}
	});

	it("lists the conflict in the full view and switches only on confirm", async () => {
		await selectTunnel(first.name);
		await connectAndPing(FULL_MFA_VIEW);

		await selectTunnel(second.name);
		await clickConnect();
		await expectConflictModal(first.name);
		await closeConflictModal("Cancel");
		await expect($(".connect-button.disconnected")).toBeDisplayed();
		await waitForActiveTunnels([first.name]);

		await clickConnect();
		await expectConflictModal(first.name);
		await closeConflictModal("Disconnect others and connect");
		await waitForActiveTunnels([second.name]);
		await waitForGatewayPing();

		await disconnect();
		await waitForActiveTunnels([]);
	});

	it("opens the conflict in the full view when connecting from the tray", async () => {
		await selectTunnel(first.name);
		await connectAndPing(FULL_MFA_VIEW);

		await selectTunnel(second.name);
		await switchToTrayView();
		await clickConnect();

		await switchToWindowLabel("full-view");
		await expectConflictModal(first.name);
		await closeConflictModal("Disconnect others and connect");
		await waitForActiveTunnels([second.name]);
		await waitForGatewayPing();

		await selectTunnel(second.name);
		await disconnect();
		await waitForActiveTunnels([]);
	});

	it("connects a tunnel whose routes do not overlap next to an active one", async () => {
		await selectTunnel(first.name);
		await connectAndPing(FULL_MFA_VIEW);

		await selectTunnel(split.name);
		await clickConnect();
		await $(".connect-button.connected").waitForDisplayed();
		await expect(conflictModal()).not.toBeDisplayed();
		await waitForActiveTunnels([first.name, split.name]);
		await waitForGatewayPing();

		await disconnect();
		await selectTunnel(first.name);
		await disconnect();
		await waitForActiveTunnels([]);
	});
});
