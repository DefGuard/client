import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { IS_MACOS, IS_WINDOWS } from "./platform.js";

const execFileAsync = promisify(execFile);

const GATEWAY_VPN_IP = process.env.GATEWAY_VPN_IP ?? "10.10.10.1";
const PING_TIMEOUT_S = 5;

// Every ping speaks its own dialect: -W is seconds on Linux but milliseconds on macOS,
// and Windows counts packets with -n and waits with -w. Getting this wrong is silent —
// a 5 ms timeout just reports the gateway as unreachable.
const pingArgs = (target: string): string[] => {
	if (IS_WINDOWS) {
		return ["-n", "1", "-w", String(PING_TIMEOUT_S * 1_000), target];
	}
	if (IS_MACOS) {
		return ["-c", "1", "-W", String(PING_TIMEOUT_S * 1_000), target];
	}
	return ["-c", "1", "-W", String(PING_TIMEOUT_S), target];
};

export const canPingGateway = async (
	target = GATEWAY_VPN_IP,
): Promise<boolean> => {
	try {
		const { stdout } = await execFileAsync("ping", pingArgs(target));
		// Windows' ping exits 0 even for "Destination host unreachable", so look for a reply.
		return IS_WINDOWS ? /TTL=/i.test(stdout) : true;
	} catch {
		return false;
	}
};
