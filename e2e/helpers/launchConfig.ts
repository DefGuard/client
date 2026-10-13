import fs from 'node:fs';
import path from 'node:path';

export type LaunchConfig = {
  files?: Record<string, string>;
  args?: string[];
};

export const launchConfigs: Record<string, LaunchConfig> = {};

export const getLaunchConfig = (specs: string[]): LaunchConfig | undefined => {
  for (const spec of specs) {
    const specName = path.basename(spec);
    if (Object.hasOwn(launchConfigs, specName)) return launchConfigs[specName];
  }
};

export const writeLaunchFiles = (
  dataDir: string,
  files: Record<string, string>,
): void => {
  const appDataDir = path.join(dataDir, 'share', 'net.defguard');
  fs.mkdirSync(appDataDir, { recursive: true });
  for (const [fileName, contents] of Object.entries(files)) {
    fs.writeFileSync(path.join(appDataDir, fileName), contents);
  }
};
