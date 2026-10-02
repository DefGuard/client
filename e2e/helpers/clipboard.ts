import { spawnSync } from "node:child_process";

export const readClipboard = (): string => {
	const result = spawnSync("xclip", ["-selection", "clipboard", "-o"], {
		encoding: "utf8",
		timeout: 5_000,
	});
	if (
		result.error &&
		"code" in result.error &&
		result.error.code === "ENOENT"
	) {
		throw new Error("xclip is missing; install it to read the clipboard.");
	}
	return result.status === 0 ? result.stdout : "";
};
