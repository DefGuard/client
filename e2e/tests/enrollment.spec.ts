import { $, expect } from "@wdio/globals";
import { resetInstances } from "../helpers/client.js";
import {
	connectAndPing,
	disconnect,
	FULL_MFA_VIEW,
	TRAY_MFA_VIEW,
} from "../helpers/connection.js";
import {
	type CoreApi,
	type EnrollmentFixture,
	loggedInCoreApi,
} from "../helpers/coreApi.js";
import {
	addInstance,
	configureTotp,
	finishEnrollment,
	selectTotpIfNeeded,
	setPassword,
} from "../helpers/enrollment.js";
import { switchToTrayView } from "../helpers/windows.js";

const refreshInstanceConfiguration = async (url: string, token: string) => {
	const actions = $(".overview-header-actions");
	await actions.waitForClickable();
	await actions.click();
	const refresh = $(".menu-item*=Refresh configuration");
	await refresh.waitForClickable();
	await refresh.click();
	const modal = $("#update-instance-modal");
	await modal.waitForDisplayed();
	await modal.$('[data-testid="field-url"]').setValue(url);
	await modal.$('[data-testid="field-token"]').setValue(token);
	const update = modal.$("button=Update");
	await update.waitForClickable();
	await update.click();
	await modal.waitForDisplayed({ reverse: true });
};

describe("enrollment", () => {
	let core: CoreApi;
	let networkId: number;
	let previousMfaStates: Map<number, boolean> | undefined;
	let fixture: EnrollmentFixture | undefined;

	beforeEach(async () => {
		previousMfaStates = undefined;
		fixture = undefined;
		core = await loggedInCoreApi();
		const networkName = process.env.NETWORK_NAME ?? "e2e";
		const network = (await core.listNetworks()).find(
			(item) => item.name === networkName,
		);
		if (!network) {
			throw new Error(`Core network "${networkName}" was not found`);
		}
		networkId = network.id;
	});

	afterEach(async () => {
		if (previousMfaStates) {
			await core.restoreLocationMfaEnabled(previousMfaStates);
		}
		await resetInstances();
		if (fixture?.ephemeral) {
			await core.deleteUser(fixture.username);
		}
	});

	it("enrolls a user without MFA and connects from the full and tray views", async () => {
		previousMfaStates = await core.disableAllLocationMfa();
		fixture = await core.createEnrollmentFixture();

		await addInstance(fixture);
		await setPassword();
		await expect($("#mfa-configuration-step")).not.toBeDisplayed();
		await finishEnrollment();

		await connectAndPing(FULL_MFA_VIEW);
		await disconnect();

		await switchToTrayView();
		await connectAndPing(TRAY_MFA_VIEW);
	});

	it("enrolls a user with TOTP MFA and connects from the full and tray views", async () => {
		previousMfaStates = await core.disableAllLocationMfa();
		await core.setLocationMfaEnabled(networkId, true);
		fixture = await core.createEnrollmentFixture();

		await addInstance(fixture);
		await setPassword();
		await selectTotpIfNeeded();
		const secret = await configureTotp();
		await finishEnrollment();
		await refreshInstanceConfiguration(
			fixture.enrollmentUrl,
			await core.startDesktopConfiguration(fixture.username),
		);

		await connectAndPing(FULL_MFA_VIEW, secret);
		await disconnect();

		await switchToTrayView();
		await connectAndPing(TRAY_MFA_VIEW, secret);
	});
});
