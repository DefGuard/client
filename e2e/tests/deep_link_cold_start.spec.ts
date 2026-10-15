import { $, browser, expect } from "@wdio/globals";
import { switchToWindowLabel } from "../helpers/windows.js";

const token = "e2e-cold-start-token";
const url = "https://cold-start.example";

describe("cold-start deep link", () => {
	it("prioritizes the deep link over provisioning config", async () => {
		await switchToWindowLabel("full-view");
		await browser.waitUntil(
			async () =>
				new URL(await browser.getUrl()).pathname === "/full/add/instance",
			{ timeout: 30_000, timeoutMsg: "Cold-start deep link was not handled" },
		);

		const currentUrl = new URL(await browser.getUrl());
		expect(currentUrl.searchParams.get("token")).toBe(token);
		expect(currentUrl.searchParams.get("url")).toBe(url);
		await expect($("#add-instance-view")).toBeDisplayed();
		await expect($('[data-testid="field-url"]')).not.toBeDisplayed();
	});
});
