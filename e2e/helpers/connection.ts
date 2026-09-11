import { $, browser } from "@wdio/globals";
import { submitTotpCode } from "./mfa.js";
import { canPingGateway } from "./tunnel.js";

export const FULL_MFA_VIEW = "#mfa-totp-view";
export const TRAY_MFA_VIEW = ".location-card-mfa-totp-view";

export const connectAndPing = async (mfaView: string, totpSecret?: string) => {
	const button = $(".connect-button");
	await button.waitForClickable();
	await button.click();

	if (totpSecret) {
		const view = $(mfaView);
		await view.waitForDisplayed();
		await submitTotpCode(
			totpSecret,
			mfaView,
			async () => {
				const verify = view.$("button=Verify");
				await verify.waitForClickable();
				await verify.click();
			},
			async () => !(await view.isDisplayed().catch(() => false)),
		);
	}

	await browser.waitUntil(() => canPingGateway(), {
		timeout: 30_000,
		interval: 2_000,
		timeoutMsg: "Could not ping the gateway through the VPN",
	});
};

export const disconnect = async () => {
	const button = $(".connect-button.connected");
	await button.waitForClickable();
	await button.click();
	await $(".connect-button.disconnected").waitForDisplayed();
};
