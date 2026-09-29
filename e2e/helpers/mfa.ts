import { $, browser } from "@wdio/globals";
import { totpCode } from "./totp.js";

const MAX_ATTEMPTS = 3;
const ACCEPT_TIMEOUT_MS = 6_000;

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
	const waitAccepted = () =>
		browser
			.waitUntil(accepted, { timeout: ACCEPT_TIMEOUT_MS, interval: 250 })
			.then(
				() => true,
				() => false,
			);

	for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
		await fillCode(scope, totpCode(secret));
		if (await waitAccepted()) return;
		await submit().catch(() => undefined);
		if (await waitAccepted()) return;
	}
	throw new Error("TOTP code was not accepted after several attempts");
};
