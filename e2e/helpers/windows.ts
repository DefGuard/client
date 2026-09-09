import { $, browser } from "@wdio/globals";

// Tauri serves the frontend from tauri://localhost on Linux and macOS but from
// http://tauri.localhost on Windows, so take the origin from wherever we already are
// rather than hardcoding it. `new URL().origin` is no help: it returns "null" for the
// non-special tauri:// scheme.
const navigateToView = async (view: string) => {
	const current = await browser.getUrl();
	const base = current.replace(/^([a-z]+:\/\/[^/]+).*$/i, "$1");
	await browser.url(`${base}/${view}/`);
	// The navigation command returns before the load commits, so the very next command
	// would still see the old route.
	await browser.waitUntil(
		async () => (await browser.getUrl()).includes(`/${view}`),
		{ timeoutMsg: `Window did not navigate to /${view}` },
	);
};

export const switchToFullView = async () => {
	for (const handle of await browser.getWindowHandles()) {
		await browser.switchToWindow(handle);
		const url = await browser.getUrl();
		if (url.includes("/full")) {
			return;
		}
		if (url.includes("/compact")) {
			await navigateToView("full");
			return;
		}
	}
	throw new Error("No full view window found");
};

export const switchToTrayView = async () => {
	await switchToFullView();
	await navigateToView("compact");
	await $("#compact-locations-page").waitForDisplayed();
};
