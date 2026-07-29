import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const GATEWAY_VPN_IP = process.env.GATEWAY_VPN_IP ?? "10.10.10.1";

export const canPingGateway = async (
	target = GATEWAY_VPN_IP,
): Promise<boolean> => {
	try {
		await execFileAsync("ping", ["-c", "1", "-W", "5", target]);
		return true;
	} catch {
		return false;
	}
};
