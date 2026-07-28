import { $, expect } from "@wdio/globals";
import {
	type CoreApi,
	type LocationMfaMode,
	loggedInCoreApi,
} from "../helpers/coreApi.js";
import { switchToFullView } from "../helpers/windows.js";
import { provisionTunnel } from "../helpers/wireguard.js";

const continueStep = async (stepId: string) => {
	const button = $(stepId).$("button=Continue");
	await button.waitForClickable();
	await button.click();
};

// Skipped: blocked by https://github.com/DefGuard/client/issues/1006
describe.skip("WireGuard tunnel", () => {
	let core: CoreApi;
	let networkId: number;
	let previousMfaMode: LocationMfaMode;

	beforeEach(async () => {
		core = await loggedInCoreApi();
		networkId = (await core.listNetworks())[0].id;
		previousMfaMode = await core.setLocationMfaMode(networkId, "disabled");
	});

	afterEach(async () => {
		await core.setLocationMfaMode(networkId, previousMfaMode);
	});

	it("adds a tunnel from a core-provisioned config", async () => {
		const config = await provisionTunnel(
			core,
			networkId,
			`e2e-tunnel-${Date.now()}`,
		);

		await switchToFullView();
		await $("#add-page-view").$("button=Add tunnel").click();
		await $("#add-tunnel-page").$("button=Add tunnel").click();

		await expect($("#general-info-step")).toBeDisplayed();
		await $('[data-testid="field-name"]').setValue(config.name);
		await $('[data-testid="field-address"]').setValue(config.address);
		await continueStep("#general-info-step");

		await expect($("#keys-step")).toBeDisplayed();
		await $('[data-testid="field-prvkey"]').setValue(config.prvkey);
		await $('[data-testid="field-pubkey"]').setValue(config.pubkey);
		await continueStep("#keys-step");

		await expect($("#vpn-server-step")).toBeDisplayed();
		await $('[data-testid="field-server_pubkey"]').setValue(
			config.serverPubkey,
		);
		await $('[data-testid="field-endpoint"]').setValue(config.endpoint);
		await $('[data-testid="field-allowed_ips"]').setValue(config.allowedIps);
		await $('[data-testid="field-persistent_keep_alive"]').setValue(
			config.keepalive,
		);
		await continueStep("#vpn-server-step");

		await expect($("#advanced-settings-step")).toBeDisplayed();
		await continueStep("#advanced-settings-step");

		await expect($("#finish-step")).toBeDisplayed();
		await expect($("#finish-step")).toHaveText("added successfully", {
			containing: true,
		});
	});
});
