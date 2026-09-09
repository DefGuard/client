import { spawnSync } from "node:child_process";
import { IS_MACOS, IS_WINDOWS } from "./platform.js";

const readCommand = (): [string, string[]] => {
	if (IS_MACOS) return ["pbpaste", []];
	if (IS_WINDOWS) {
		return ["powershell", ["-NoProfile", "-Command", "Get-Clipboard"]];
	}
	return ["xclip", ["-selection", "clipboard", "-o"]];
};

export const readClipboard = (): string => {
	const [command, args] = readCommand();
	const result = spawnSync(command, args, {
		encoding: "utf8",
		timeout: 5_000,
	});
	// Get-Clipboard normalises line endings to CRLF; callers compare against webview text.
	return result.status === 0 ? result.stdout.replace(/\r\n/g, "\n") : "";
};
