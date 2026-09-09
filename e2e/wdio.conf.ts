import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { IS_LINUX, IS_WINDOWS } from "./helpers/platform.js";

const here = import.meta.dirname;

const TEST_TIMEOUT_MS = 120_000;
const WAIT_FOR_TIMEOUT_MS = 15_000;
// The app has to boot and bring up its embedded WebDriver server; a cold start on a
// loaded CI box is slow.
const APP_START_TIMEOUT_MS = 120_000;

const envFile = path.resolve(here, ".env");
if (fs.existsSync(envFile)) {
	process.loadEnvFile(envFile);
}

const binaryName = IS_WINDOWS ? "defguard-client.exe" : "defguard-client";
const clientBinary =
	process.env.CLIENT_BINARY ??
	path.resolve(here, "../src-tauri/target/release", binaryName);
// Single-instance keys on the Tauri app identifier, not the executable name, so a client
// built under any name takes over any other. Look for both the default name and whatever
// CLIENT_BINARY happens to be called.
const processNames = [...new Set([binaryName, path.basename(clientBinary)])];

// Deterministic rather than mkdtemp: this module is loaded by both the launcher and the
// worker, and both have to agree on the path. The launcher spawns the client, so the
// client inherits the environment we set below.
const dataDir = path.join(os.tmpdir(), "defguard-e2e");

// The launcher spawns the client and owns its lifetime; the worker only drives it over
// WebDriver. Anything that kills clients or wipes the profile has to stay on this side of
// the fence, or it takes down the app under test mid-session.
const isWorker = Boolean(process.env.WDIO_WORKER_ID);

// Point the client at a throwaway profile. Tauri derives its data, config and log
// directories from a different set of variables on each platform, so override all of them.
const isolateAppDirs = () => {
	if (IS_WINDOWS) {
		process.env.APPDATA = path.join(dataDir, "Roaming");
		process.env.LOCALAPPDATA = path.join(dataDir, "Local");
	} else if (IS_LINUX) {
		process.env.XDG_DATA_HOME = path.join(dataDir, "share");
		process.env.XDG_CONFIG_HOME = path.join(dataDir, "config");
		process.env.XDG_CACHE_HOME = path.join(dataDir, "cache");
	} else {
		// macOS hangs everything off the home directory (~/Library/...).
		process.env.HOME = path.join(dataDir, "home");
	}
	process.env.DEFGUARD_CLIENT_WELCOME_SKIP = "1";
};

isolateAppDirs();

const killLeftoverClients = () => {
	if (IS_WINDOWS) {
		for (const name of processNames) {
			spawnSync("taskkill", ["/f", "/im", name], { stdio: "ignore" });
		}
		return;
	}
	spawnSync("pkill", ["-f", clientBinary]);
};

// Linux gets a kernel WireGuard interface per connection, and a test that fails while
// connected leaves it behind. macOS and Windows tunnel through adapters owned by the
// system extension and the service respectively, which they tear down themselves.
const cleanupWireguardInterfaces = () => {
	if (!IS_LINUX) return;
	const listed = spawnSync("ip", ["-j", "link", "show", "type", "wireguard"], {
		encoding: "utf8",
	});
	const links = JSON.parse(listed.stdout || "[]") as Array<{ ifname: string }>;
	for (const { ifname } of links) {
		if (!/^wg\d+$/.test(ifname)) continue;
		if (spawnSync("ip", ["link", "delete", ifname]).status !== 0) {
			spawnSync("sudo", ["-n", "ip", "link", "delete", ifname]);
		}
	}
};

// Only a build made with the `e2e` Cargo feature carries the embedded WebDriver server.
// Without it the client starts up perfectly well and simply never answers, which the
// service can only report as a start-up timeout, so check the binary up front.
const EMBEDDED_DRIVER_MARKER = "TAURI_WEBDRIVER_PORT";

const hasEmbeddedDriver = (binary: string): boolean => {
	const marker = Buffer.from(EMBEDDED_DRIVER_MARKER);
	const chunkSize = 1 << 20;
	const overlap = marker.length - 1;
	const buffer = Buffer.alloc(chunkSize + overlap);
	const fd = fs.openSync(binary, "r");
	try {
		let carried = 0;
		for (;;) {
			const read = fs.readSync(fd, buffer, carried, chunkSize, null);
			if (read === 0) return false;
			const end = carried + read;
			if (buffer.subarray(0, end).includes(marker)) return true;
			// Carry the tail over so a marker straddling two chunks still matches.
			carried = Math.min(overlap, end);
			buffer.copy(buffer, 0, end - carried, end);
		}
	} finally {
		fs.closeSync(fd);
	}
};

// Tauri's single-instance plugin makes a second client hand its arguments to the first and
// exit, so any client we did not start has to be gone before the run. Report it rather than
// killing it: on a developer machine that process is a real VPN session.
const foreignClientPids = (): string[] => {
	const pids = processNames.flatMap((name) => {
		if (IS_WINDOWS) {
			const listed = spawnSync(
				"tasklist",
				["/fi", `imagename eq ${name}`, "/nh", "/fo", "csv"],
				{ encoding: "utf8" },
			);
			return (listed.stdout ?? "")
				.split("\n")
				.map((line) => line.match(/^"[^"]+","(\d+)"/)?.[1])
				.filter((pid): pid is string => Boolean(pid));
		}
		const listed = spawnSync("pgrep", ["-x", name], { encoding: "utf8" });
		return (listed.stdout ?? "").split("\n").filter(Boolean);
	});
	return [...new Set(pids)];
};

const cleanup = () => {
	killLeftoverClients();
	cleanupWireguardInterfaces();
};

const findLogDirs = (root: string, found: string[] = []): string[] => {
	let entries: fs.Dirent[];
	try {
		entries = fs.readdirSync(root, { withFileTypes: true });
	} catch {
		return found;
	}
	for (const entry of entries) {
		if (!entry.isDirectory()) continue;
		const full = path.join(root, entry.name);
		if (entry.name.toLowerCase() === "logs") {
			found.push(full);
			continue;
		}
		findLogDirs(full, found);
	}
	return found;
};

// The client logs into the throwaway profile of the session, which is deleted once the
// session ends. Keep a copy around, as it holds the client side of any connection failure.
// The log directory sits somewhere different on each platform, so go looking for it.
const preserveClientLogs = () => {
	for (const source of findLogDirs(dataDir)) {
		const label = path.relative(dataDir, source).replace(/[\\/]+/g, "-");
		const target = path.join(here, "logs", `${label}-${Date.now()}`);
		try {
			fs.cpSync(source, target, { recursive: true });
		} catch (error) {
			console.error(`Failed to preserve client logs from ${source}:`, error);
		}
	}
};

const assertClientIsRunnable = () => {
	if (!fs.existsSync(clientBinary)) {
		throw new Error(`Client binary not found at ${clientBinary}`);
	}
	if (!hasEmbeddedDriver(clientBinary)) {
		throw new Error(
			`${clientBinary} was built without the \`e2e\` Cargo feature, so it has no ` +
				"embedded WebDriver server to connect to. Rebuild with " +
				"`cargo tauri build --features e2e`.",
		);
	}
	const foreign = foreignClientPids();
	if (foreign.length > 0) {
		throw new Error(
			`Another Defguard client is already running (PID ${foreign.join(", ")}). It would ` +
				"take over the client under test, which then exits without ever starting its " +
				"WebDriver server. Quit it first.",
		);
	}
};

// Runs before any hook, so the client the service is about to spawn starts from a clean
// profile and is not mistaken for a leftover. A hook is too late for the preconditions:
// throwing from onPrepare is only logged, and the service spawns the client anyway.
if (!isWorker) {
	cleanup();
	assertClientIsRunnable();
	fs.rmSync(dataDir, { recursive: true, force: true });
	fs.mkdirSync(dataDir, { recursive: true });

	process.on("exit", cleanup);
	for (const signal of ["SIGINT", "SIGTERM"] as const) {
		process.on(signal, () => {
			cleanup();
			process.exit(130);
		});
	}
}

export const config: WebdriverIO.Config = {
	runner: "local",
	logLevel: "info",
	specs: ["./tests/**/*.spec.ts"],
	maxInstances: 1,
	services: [
		[
			"@wdio/tauri-service",
			{
				driverProvider: "embedded",
				startTimeout: APP_START_TIMEOUT_MS,
				// Without this the client's stderr is swallowed, so a client that dies on
				// startup shows up only as a start-up timeout.
				captureBackendLogs: true,
			},
		],
	],
	capabilities: [
		{
			browserName: "tauri",
			"wdio:maxInstances": 1,
			"tauri:options": { application: clientBinary },
		} as WebdriverIO.Capabilities,
	],
	reporters: ["spec"],
	framework: "mocha",
	mochaOpts: { ui: "bdd", timeout: TEST_TIMEOUT_MS },
	waitforTimeout: WAIT_FOR_TIMEOUT_MS,
	connectionRetryTimeout: 120_000,
	connectionRetryCount: 2,

	afterTest: () => cleanupWireguardInterfaces(),

	onComplete: () => {
		cleanup();
		preserveClientLogs();
		fs.rmSync(dataDir, {
			recursive: true,
			force: true,
			maxRetries: 5,
			retryDelay: 100,
		});
	},
};
