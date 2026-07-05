import { type ChildProcess, spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const here = import.meta.dirname;

const DRIVER_PORT = 4444;
const DRIVER_READY_DELAY_MS = 1_000;
const TEST_TIMEOUT_MS = 120_000;
const WAIT_FOR_TIMEOUT_MS = 15_000;

const envFile = path.resolve(here, '.env');
if (fs.existsSync(envFile)) {
  process.loadEnvFile(envFile);
}

const clientBinary =
  process.env.CLIENT_BINARY ??
  path.resolve(here, '../src-tauri/target/release/defguard-client');
const tauriDriverBinary =
  process.env.TAURI_DRIVER ?? path.join(os.homedir(), '.cargo', 'bin', 'tauri-driver');
const nativeDriver = process.env.NATIVE_DRIVER;

let tauriDriver: ChildProcess | undefined;
let dataDir: string | undefined;

const killLeftoverClients = () => {
  spawnSync('pkill', ['-f', clientBinary]);
};

const cleanupWireguardInterfaces = () => {
  const listed = spawnSync('ip', ['-j', 'link', 'show', 'type', 'wireguard'], {
    encoding: 'utf8',
  });
  const links = JSON.parse(listed.stdout || '[]') as Array<{ ifname: string }>;
  for (const { ifname } of links) {
    if (!/^wg\d+$/.test(ifname)) continue;
    if (spawnSync('ip', ['link', 'delete', ifname]).status !== 0) {
      spawnSync('sudo', ['-n', 'ip', 'link', 'delete', ifname]);
    }
  }
};

const cleanup = () => {
  tauriDriver?.kill();
  tauriDriver = undefined;
  killLeftoverClients();
  cleanupWireguardInterfaces();
};

process.on('exit', cleanup);
for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.on(signal, () => {
    cleanup();
    process.exit(130);
  });
}

export const config: WebdriverIO.Config = {
  runner: 'local',
  hostname: '127.0.0.1',
  port: DRIVER_PORT,
  logLevel: 'info',
  specs: ['./tests/**/*.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      'wdio:maxInstances': 1,
      'tauri:options': { application: clientBinary },
    } as WebdriverIO.Capabilities,
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: TEST_TIMEOUT_MS },
  waitforTimeout: WAIT_FOR_TIMEOUT_MS,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 2,

  onPrepare: () => {
    if (!fs.existsSync(clientBinary)) {
      throw new Error(
        `Client binary not found at ${clientBinary}. Build it with ` +
        '`pnpm tauri build` or set CLIENT_BINARY.',
      );
    }
  },

  beforeSession: () => {
    killLeftoverClients();
    cleanupWireguardInterfaces();
    dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'defguard-e2e-'));
    tauriDriver = spawn(
      tauriDriverBinary,
      nativeDriver ? ['--native-driver', nativeDriver] : [],
      {
        stdio: ['ignore', 'inherit', 'inherit'],
        env: {
          ...process.env,
          XDG_DATA_HOME: path.join(dataDir, 'share'),
          XDG_CONFIG_HOME: path.join(dataDir, 'config'),
          XDG_CACHE_HOME: path.join(dataDir, 'cache'),
        },
      },
    );
    tauriDriver.on('error', (error) => {
      console.error('tauri-driver failed to start:', error);
      process.exit(1);
    });
    return new Promise((resolve) => setTimeout(resolve, DRIVER_READY_DELAY_MS));
  },

  afterTest: () => cleanupWireguardInterfaces(),

  afterSession: () => {
    cleanup();
    if (dataDir) {
      fs.rmSync(dataDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
      dataDir = undefined;
    }
  },

  onComplete: cleanup,
};
