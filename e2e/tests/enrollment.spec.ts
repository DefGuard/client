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
	type LocationMfaMode,
	loggedInCoreApi,
} from "../helpers/coreApi.js";
import {
	addInstance,
	configureTotp,
	finishEnrollment,
	setPassword,
} from "../helpers/enrollment.js";
import { switchToTrayView } from "../helpers/windows.js";

describe("enrollment", () => {
	let core: CoreApi;
	let networkId: number;
	let previousMfaMode: LocationMfaMode;
	let fixture: EnrollmentFixture;

	beforeEach(async () => {
		core = await loggedInCoreApi();
		networkId = (await core.listNetworks())[0].id;
	});

	afterEach(async () => {
		await core.setLocationMfaMode(networkId, previousMfaMode);
		await resetInstances();
		if (fixture?.ephemeral) {
			await core.deleteUser(fixture.username);
		}
	});

	it("enrolls a user without MFA and connects from the full and tray views", async () => {
		previousMfaMode = await core.setLocationMfaMode(networkId, "disabled");
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
		previousMfaMode = await core.setLocationMfaMode(networkId, "internal");
		fixture = await core.createEnrollmentFixture();

		await addInstance(fixture);
		await setPassword();
		const secret = await configureTotp();
		await finishEnrollment();

		await connectAndPing(FULL_MFA_VIEW, secret);
		await disconnect();

		await switchToTrayView();
		await connectAndPing(TRAY_MFA_VIEW, secret);
	});
});
