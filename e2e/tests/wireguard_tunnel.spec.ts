import { $, browser, expect } from "@wdio/globals";
import {
	connectAndPing,
	disconnect,
	FULL_MFA_VIEW,
	TRAY_MFA_VIEW,
} from "../helpers/connection.js";
import {
	type CoreApi,
	type LocationMfaMode,
	loggedInCoreApi,
} from "../helpers/coreApi.js";
import { switchToFullView, switchToTrayView } from "../helpers/windows.js";
import { provisionTunnel, type TunnelConfig } from "../helpers/wireguard.js";

const field = (name: string) => $(`[data-testid="field-${name}"]`);

const clearField = async (name: string) => {
	const input = field(name);
	await input.waitForClickable();
	await input.click();
	await browser.keys(["Control", "a"]);
	await browser.keys(["Backspace"]);
	await expect(input).toHaveValue("");
};

const setField = async (name: string, value: string) => {
	await clearField(name);
	await field(name).addValue(value);
	await expect(field(name)).toHaveValue(value);
};

const continueStep = async (stepId: string) => {
	const button = $(stepId).$("button=Continue");
	await button.waitForClickable();
	await button.click();
};

const selectTunnel = async (name: string) => {
	await switchToFullView();
	const overviewLink = $('a[href="/full/overview"]');
	await overviewLink.waitForClickable();
	await overviewLink.click();
	await $("#overview-page").waitForDisplayed();
	const item = $(".overview-selection").$(`button=${name}`);
	await item.waitForClickable();
	await item.click();
};

const openTunnelAction = async (action: string) => {
	const actions = $(".overview-header-actions");
	await actions.waitForClickable();
	await actions.click();
	const item = $(`.menu-item*=${action}`);
	await item.waitForClickable();
	await item.click();
};

const openTunnelEditModal = async () => {
	await openTunnelAction("Edit");
	await $("#update-tunnel-modal").waitForDisplayed();
};

const detailsRow = (label: string) =>
	$("#location-details-page").$(`.row*=${label}`);

const openTunnelDetails = async () => {
	const info = $("#overview-page .info-btn");
	await info.waitForClickable();
	await info.click();
	await $("#location-details-page").waitForDisplayed();
};

const closeTunnelDetails = async () => {
	const back = $("#location-details-page").$("button=Back");
	await back.waitForClickable();
	await back.click();
	await $("#overview-page").waitForDisplayed();
};

const deleteTunnel = async (name: string) => {
	await selectTunnel(name);
	await openTunnelAction("Delete");
	const confirm = $("#confirm-modal").$("button=Delete tunnel");
	await confirm.waitForClickable();
	await confirm.click();
	await $("#confirm-modal").waitForDisplayed({ reverse: true });
};

const submitTunnelEdit = async () => {
	const update = $("#update-tunnel-modal").$("button=Update");
	await update.waitForClickable();
	await update.click();
	await $("#update-tunnel-modal").waitForDisplayed({ reverse: true });
};

describe("WireGuard tunnel", () => {
	let core: CoreApi;
	let networkId: number;
	let previousMfaMode: LocationMfaMode;
	let config: TunnelConfig;

	before(async () => {
		core = await loggedInCoreApi();
		networkId = (await core.listNetworks())[0].id;
		previousMfaMode = await core.setLocationMfaMode(networkId, "disabled");
		config = await provisionTunnel(core, networkId, `e2e-tunnel-${Date.now()}`);
	});

	after(async () => {
		await core.setLocationMfaMode(networkId, previousMfaMode);
		await deleteTunnel(config.name);
		await core.deleteDevice(config.deviceId);
	});

	it("adds a tunnel from a core-provisioned config", async () => {
		await switchToFullView();
		await $("#add-page-view").$("button=Add tunnel").click();
		await $("#add-tunnel-page").$("button=Add tunnel").click();

		await expect($("#general-info-step")).toBeDisplayed();
		await setField("name", config.name);
		await setField("address", config.address);
		await continueStep("#general-info-step");

		await expect($("#keys-step")).toBeDisplayed();
		await setField("prvkey", config.prvkey);
		await setField("pubkey", config.pubkey);
		await continueStep("#keys-step");

		await expect($("#vpn-server-step")).toBeDisplayed();
		await setField("server_pubkey", config.serverPubkey);
		await setField("endpoint", config.endpoint);
		await setField("allowed_ips", config.allowedIps);
		await setField("dns", config.dns || "1.1.1.1");
		await clearField("dns");
		await expect($("#vpn-server-step")).not.toHaveText("Invalid input", {
			containing: true,
		});
		await continueStep("#vpn-server-step");

		await expect($("#advanced-settings-step")).toBeDisplayed();
		await continueStep("#advanced-settings-step");

		await expect($("#finish-step")).toBeDisplayed();
		await expect($("#finish-step")).toHaveText("added successfully", {
			containing: true,
		});

		await $("#finish-step").$("button=Got it").click();
		await expect($("#overview-page")).toHaveText(config.name, {
			containing: true,
		});
	});

	it("edits an existing tunnel and clears its optional fields", async () => {
		const dns = config.dns || "8.8.8.8";
		await selectTunnel(config.name);

		await openTunnelEditModal();
		await setField("dns", dns);
		await setField("post_up", "echo up");
		await submitTunnelEdit();

		await openTunnelDetails();
		await expect(detailsRow("DNS servers")).toHaveText(dns, {
			containing: true,
		});
		await closeTunnelDetails();

		await openTunnelEditModal();
		await expect(field("dns")).toHaveValue(dns);
		await expect(field("post_up")).toHaveValue("echo up");

		await clearField("dns");
		await clearField("post_up");
		await expect($("#update-tunnel-modal")).not.toHaveText("Invalid input", {
			containing: true,
		});
		await submitTunnelEdit();

		await openTunnelEditModal();
		await expect(field("dns")).toHaveValue("");
		await expect(field("post_up")).toHaveValue("");

		await $("#update-tunnel-modal").$("button=Cancel").click();
		await $("#update-tunnel-modal").waitForDisplayed({ reverse: true });
	});

	it("connects the tunnel and pings the gateway from the full view", async () => {
		await selectTunnel(config.name);

		await connectAndPing(FULL_MFA_VIEW);
		await disconnect();
	});

	it("connects the tunnel and pings the gateway from the tray view", async () => {
		await switchToTrayView();

		await connectAndPing(TRAY_MFA_VIEW);
		await disconnect();
	});
});
