import { $, browser, expect } from "@wdio/globals";
import { readClipboard } from "../helpers/clipboard.js";
import { switchToFullView } from "../helpers/windows.js";

const openActionsMenu = async () => {
	const actions = $("#log-page-view").$("button=Actions");
	await actions.waitForClickable();
	await actions.click();
};

describe("logs", () => {
	beforeEach(async () => {
		await switchToFullView();
		const logLink = $('a[href="/full/log"]');
		await logLink.waitForClickable();
		await logLink.click();
		await expect($("#log-page-view")).toBeDisplayed();
		await $("#log-page-view .log-container p").waitForExist({
			timeout: 15_000,
		});
	});

	it("copies logs to the clipboard", async () => {
		const firstLine = $("#log-page-view .log-container p");
		const sample = (
			(await firstLine.getProperty("textContent")) as string
		).trim();
		await openActionsMenu();
		const copy = $(".menu-item*=Copy to Clipboard");
		await copy.waitForClickable();
		await copy.click();
		await browser.waitUntil(() => readClipboard().includes(sample), {
			timeout: 10_000,
			timeoutMsg: "Clipboard does not contain the logs after copying",
		});
	});

	it("offers a logs download", async () => {
		await openActionsMenu();
		await expect($(".menu-item*=Download")).toBeDisplayed();
	});
});
