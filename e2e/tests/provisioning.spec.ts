import fs from "node:fs";
import path from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { invoke } from "../helpers/client.js";
import { provisioningFixture } from "../helpers/launchConfig.js";
import { switchToWindowLabel } from "../helpers/windows.js";

const { enrollmentToken, enrollmentUrl } = provisioningFixture;

describe("provisioning", () => {
	it("opens the prefilled instance form from launch config", async () => {
		await switchToWindowLabel("full-view");
		await browser.waitUntil(
			async () =>
				new URL(await browser.getUrl()).pathname === "/full/add/instance",
			{ timeout: 10_000, timeoutMsg: "Provisioning form did not open" },
		);

		const currentUrl = new URL(await browser.getUrl());
		expect(currentUrl.searchParams.get("token")).toBe(enrollmentToken);
		expect(currentUrl.searchParams.get("url")).toBe(enrollmentUrl);
		await expect($("#add-instance-view")).toBeDisplayed();
		await expect($('[data-testid="field-url"]')).not.toBeDisplayed();
	});

	it("does not restore provisioning after the saved instance is deleted", async () => {
		await switchToWindowLabel("full-view");
		const saved = await invoke<{ instance: { id: number } }>(
			"save_device_config",
			{
				privateKey: "e2e-placeholder-private-key",
				response: {
					device: {
						id: 1,
						name: "e2e-device",
						pubkey: "e2e-placeholder-public-key",
						user_id: 1,
						created_at: 0,
					},
					configs: [],
					instance: {
						id: "00000000-0000-4000-8000-000000000001",
						name: "e2e-instance",
						url: "https://core.example",
						proxy_url: "https://edge.example",
						username: "e2e",
					},
				},
			},
		);
		const dataDir = process.env.E2E_DATA_DIR;
		if (!dataDir) throw new Error("E2E_DATA_DIR is required");
		expect(
			fs.existsSync(
				path.join(dataDir, "share", "net.defguard", "provisioning.json"),
			),
		).toBe(true);

		await browser.url("tauri://localhost/full/");
		await browser.waitUntil(
			async () => new URL(await browser.getUrl()).pathname === "/full/overview",
			{ timeout: 10_000, timeoutMsg: "Expected the saved-instance overview" },
		);

		await invoke("delete_instance", { instanceId: saved.instance.id });
		await browser.url("tauri://localhost/full/");
		await browser.waitUntil(
			async () => new URL(await browser.getUrl()).pathname === "/full/add",
			{ timeout: 10_000, timeoutMsg: "Expected the empty-instance add page" },
		);
		await expect($("#add-instance-view")).not.toBeDisplayed();
	});
});
