import fs from "node:fs";
import path from "node:path";

export type LaunchConfig = {
	files?: Record<string, string>;
	args?: string[];
};

type ProvisioningFixture = { enrollmentUrl: string; enrollmentToken: string };
type DeepLinkFixture = { url: string; token: string };

export const provisioningFixture: ProvisioningFixture = {
	enrollmentUrl: "https://enroll.example",
	enrollmentToken: "e2e-provisioning-token",
};

export const coldStartFixture: {
	provisioning: ProvisioningFixture;
	deepLink: DeepLinkFixture;
} = {
	provisioning: {
		enrollmentUrl: "https://provisioning.example",
		enrollmentToken: "e2e-provisioning-token",
	},
	deepLink: {
		url: "https://cold-start.example",
		token: "e2e-cold-start-token",
	},
};

export const provisioningFile = ({
	enrollmentUrl,
	enrollmentToken,
}: ProvisioningFixture): string =>
	JSON.stringify({
		enrollment_url: enrollmentUrl,
		enrollment_token: enrollmentToken,
	});

export const deepLinkArg = ({ url, token }: DeepLinkFixture): string =>
	`defguard://addinstance?token=${encodeURIComponent(token)}&url=${encodeURIComponent(url)}`;

export const launchConfigs: Record<string, LaunchConfig> = {
	"deep_link_cold_start.spec.ts": {
		files: {
			"provisioning.json": provisioningFile(coldStartFixture.provisioning),
		},
		args: [deepLinkArg(coldStartFixture.deepLink)],
	},
	"provisioning.spec.ts": {
		files: { "provisioning.json": provisioningFile(provisioningFixture) },
	},
};

export const getLaunchConfig = (specs: string[]): LaunchConfig | undefined => {
	const configured = specs
		.map((spec) => path.basename(spec))
		.filter((specName) => Object.hasOwn(launchConfigs, specName));
	if (configured.length > 0 && specs.length > 1) {
		throw new Error(
			`Specs with a launch config must run in their own session: ${configured.join(", ")}`,
		);
	}
	return configured.length > 0 ? launchConfigs[configured[0]] : undefined;
};

export const assertLaunchConfigSpecsExist = (testsDir: string): void => {
	for (const specName of Object.keys(launchConfigs)) {
		if (!fs.existsSync(path.join(testsDir, specName))) {
			throw new Error(`Launch config refers to missing spec tests/${specName}`);
		}
	}
};

export const writeLaunchFiles = (
	dataDir: string,
	files: Record<string, string>,
): void => {
	const appDataDir = path.join(dataDir, "share", "net.defguard");
	fs.mkdirSync(appDataDir, { recursive: true });
	for (const [fileName, contents] of Object.entries(files)) {
		fs.writeFileSync(path.join(appDataDir, fileName), contents);
	}
};
