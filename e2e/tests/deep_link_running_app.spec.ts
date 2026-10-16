import { spawnSync } from "node:child_process";
import { $, browser, expect } from "@wdio/globals";
import { switchToWindowLabel } from "../helpers/windows.js";

const token = "e2e-running-app-token";
const url = "https://running.example";

describe("running-app deep link", () => {
	it("forwards the link to the running client", async () => {
		await switchToWindowLabel("full-view");
		await browser.waitUntil(
			async () => new URL(await browser.getUrl()).pathname === "/full/add",
			{ timeout: 30_000, timeoutMsg: "Full view did not open" },
		);
		await expect($("#add-page-view")).toBeDisplayed();

		const clientBinary = process.env.CLIENT_BINARY;
		if (!clientBinary) {
			throw new Error("CLIENT_BINARY was not set by the e2e session");
		}
		if (!process.env.DBUS_SESSION_BUS_ADDRESS) {
			throw new Error("The e2e process has no D-Bus session");
		}

		const secondInstance = spawnSync(
			clientBinary,
			[`defguard://addinstance?token=${token}&url=${encodeURIComponent(url)}`],
			{ env: { ...process.env }, stdio: "ignore", timeout: 15_000 },
		);
		if (secondInstance.error) throw secondInstance.error;
		expect(secondInstance.status).toBe(0);
		expect(secondInstance.signal).toBeNull();

		await browser.waitUntil(
			async () =>
				new URL(await browser.getUrl()).pathname === "/full/add/instance",
			{
				timeout: 15_000,
				timeoutMsg: "Running client did not handle the deep link",
			},
		);
		const currentUrl = new URL(await browser.getUrl());
		expect(currentUrl.searchParams.get("token")).toBe(token);
		expect(currentUrl.searchParams.get("url")).toBe(url);
		await expect($("#add-instance-view")).toBeDisplayed();
		await expect($('[data-testid="field-url"]')).not.toBeDisplayed();
	});
});
