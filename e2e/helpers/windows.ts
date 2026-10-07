import { $, browser } from "@wdio/globals";

export const switchToFullView = async () => {
	for (const handle of await browser.getWindowHandles()) {
		await browser.switchToWindow(handle);
		const url = await browser.getUrl();
		if (url.includes("/full")) {
			return;
		}
		if (url.includes("/compact")) {
			await browser.url("tauri://localhost/full/");
			return;
		}
	}
	throw new Error("No full view window found");
};

export const switchToTrayView = async () => {
	await switchToFullView();
	await browser.url("tauri://localhost/compact/");
	await $("#compact-locations-page").waitForDisplayed();
};

export const selectTunnel = async (name: string) => {
	await switchToFullView();
	const overviewLink = $('a[href="/full/overview"]');
	await overviewLink.waitForClickable();
	await overviewLink.click();
	await $("#overview-page").waitForDisplayed();
	const item = $(".overview-selection").$(`button=${name}`);
	await item.waitForClickable();
	await item.click();
};

export const switchToWindowLabel = async (label: string) => {
	for (const handle of await browser.getWindowHandles()) {
		await browser.switchToWindow(handle);
		const current = await browser.execute(
			() =>
				(
					window as unknown as {
						__TAURI_INTERNALS__: {
							metadata: { currentWindow: { label: string } };
						};
					}
				).__TAURI_INTERNALS__.metadata.currentWindow.label,
		);
		if (current === label) {
			return;
		}
	}
	throw new Error(`No window with label ${label} found`);
};
