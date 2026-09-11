import { spawnSync } from "node:child_process";

export const readClipboard = (): string => {
	const result = spawnSync("xclip", ["-selection", "clipboard", "-o"], {
		encoding: "utf8",
		timeout: 5_000,
	});
	return result.status === 0 ? result.stdout : "";
};
