import { $, browser } from "@wdio/globals";
import { totpCode } from "./totp.js";

const MAX_ATTEMPTS = 3;

const fillCode = async (scope: string, code: string) => {
	const input = $(`${scope} .code-input input`);
	await input.click();
	await input.setValue(code);
};

export const submitTotpCode = async (
	secret: string,
	scope: string,
	submit: () => Promise<void>,
	accepted: () => Promise<boolean>,
) => {
	for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
		await fillCode(scope, totpCode(secret));
		await submit();
		try {
			await browser.waitUntil(accepted, {
				timeout: 6_000,
				interval: 500,
				timeoutMsg: "TOTP code was not accepted after several attempts",
			});
			return;
		} catch (error) {
			if (attempt === MAX_ATTEMPTS) throw error;
		}
	}
};
