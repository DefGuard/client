#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { loggedInCoreApi } from "../helpers/coreApi.js";
import { locationSpec, resetTestNetwork } from "../helpers/locationSpec.js";

const envFile = path.resolve(import.meta.dirname, "../.env");
if (fs.existsSync(envFile)) {
	process.loadEnvFile(envFile);
}

const requireEnv = (name: string): string => {
	const value = process.env[name];
	if (!value) {
		console.error(`Missing required environment variable ${name}.`);
		process.exit(1);
	}
	return value;
};

const CORE_URL = requireEnv("CORE_URL");
requireEnv("CORE_ADMIN_PASSWORD");
requireEnv("NETWORK_ENDPOINT");

const waitForCore = async () => {
	const deadline = Date.now() + 120_000;
	while (Date.now() < deadline) {
		try {
			const res = await fetch(`${CORE_URL}/api/v1/health`);
			if (res.status === 200) return;
		} catch {
			// not up yet
		}
		await new Promise((resolve) => setTimeout(resolve, 2000));
	}
	throw new Error(`Core did not become healthy at ${CORE_URL} within 120s`);
};

console.log(`Waiting for core at ${CORE_URL}...`);
await waitForCore();

console.log("Logging in as admin...");
const core = await loggedInCoreApi().catch((error) => {
	if (String(error).includes("405")) {
		console.error(`\nCore at ${CORE_URL} is still in initial-setup mode`);
		process.exit(1);
	}
	throw error;
});

const spec = locationSpec();
if ((await core.listNetworks()).length === 0) {
	const network = await core.createNetwork({ ...spec, mfa_flows: [] });
	console.log(`Network created with id ${network.id}`);
}

const networkId = await resetTestNetwork(core);
console.log(
	`Network "${spec.name}" (id ${networkId}) matches the e2e location spec`,
);

const gateways = await core.listGateways(networkId);
if (gateways.length === 0) {
	process.exit(1);
}
console.log(`Gateway connected: ${gateways[0].name}`);
