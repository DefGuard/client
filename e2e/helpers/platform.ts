export const IS_LINUX = process.platform === "linux";
export const IS_MACOS = process.platform === "darwin";
export const IS_WINDOWS = process.platform === "win32";

// The modifier that selects all, copies and pastes: Command on macOS, Control elsewhere.
// WebdriverIO maps "Command" onto the Meta key, which is what the webview expects.
export const PRIMARY_MODIFIER = IS_MACOS ? "Command" : "Control";
