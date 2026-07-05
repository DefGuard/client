#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

const envFile = path.resolve(import.meta.dirname, '../.env');
if (fs.existsSync(envFile)) {
  process.loadEnvFile(envFile);
}

const CORE_URL = requireEnv('CORE_URL');
const ADMIN_USER = process.env.CORE_ADMIN_USER ?? 'admin';
const ADMIN_PASSWORD = requireEnv('CORE_ADMIN_PASSWORD');

function requireEnv(name) {
  const value = process.env[name];
  if (!value) {
    console.error(`Missing required environment variable ${name}.`);
    process.exit(1);
  }
  return value;
}

const NETWORK = {
  name: process.env.NETWORK_NAME ?? 'e2e',
  address: process.env.NETWORK_ADDRESS ?? '10.10.10.1/24',
  endpoint: requireEnv('NETWORK_ENDPOINT'),
  port: Number(process.env.NETWORK_PORT ?? 50051),
  allowed_ips: process.env.NETWORK_ALLOWED_IPS ?? '10.10.10.0/24',
  dns: null,
  mtu: 1420,
  fwmark: 0,
  allow_all_groups: true,
  allowed_groups: [],
  keepalive_interval: 25,
  peer_disconnect_threshold: 300,
  acl_enabled: false,
  acl_default_allow: false,
  location_mfa_mode: 'disabled',
  service_location_mode: 'disabled',
};

async function waitForCore() {
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
}

let cookie = '';

async function api(method, apiPath, body) {
  const res = await fetch(`${CORE_URL}${apiPath}`, {
    method,
    redirect: 'manual',
    headers: {
      'Content-Type': 'application/json',
      ...(cookie ? { Cookie: cookie } : {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (res.status >= 300 && res.status < 400) {
    throw new Error(
      `${method} ${apiPath} redirected (${res.status}) to ${res.headers.get('location')} — check CORE_URL`,
    );
  }
  if (!res.ok) {
    const allow = res.headers.get('allow');
    throw new Error(
      `${method} ${apiPath} failed: ${res.status}` +
      (allow ? ` (Allow: ${allow})` : '') +
      ` ${await res.text()}`,
    );
  }
  return res;
}

console.log(`Waiting for core at ${CORE_URL}...`);
await waitForCore();

console.log('Logging in as admin...');
let loginRes;
try {
  loginRes = await api('POST', '/api/v1/auth', {
    username: ADMIN_USER,
    password: ADMIN_PASSWORD,
  });
} catch (error) {
  if (String(error).includes('405')) {
    console.error(
      `\nCore at ${CORE_URL} is still in initial-setup mode. ` +
      `Open ${CORE_URL} in a browser and complete the setup wizard ` +
      '(all the way through the final step — core restarts afterwards), ' +
      'then re-run this script.\n',
    );
    process.exit(1);
  }
  throw error;
}
const setCookie = loginRes.headers.get('set-cookie');
if (!setCookie) throw new Error('Login did not return a session cookie');
cookie = setCookie.split(';')[0];

const existing = await (await api('GET', '/api/v1/network')).json();
let network = existing.find((n) => n.name === NETWORK.name);
if (network) {
  console.log(`Network "${NETWORK.name}" already exists with id ${network.id}`);
} else {
  console.log('Creating VPN network...');
  network = await (await api('POST', '/api/v1/network', NETWORK)).json();
  console.log(`Network created with id ${network.id}`);
}

const gateways = await (
  await api('GET', `/api/v1/network/${network.id}/gateways`)
).json();
if (gateways.length === 0) {
  console.error(
    `\nNo gateway is connected to network "${NETWORK.name}" (id ${network.id}).\n` +
    'Gateway setup is intentionally not automated — add and connect one yourself,\n' +
    `then re-run this script. See https://docs.defguard.net for the deployment guide.\n`,
  );
  process.exit(1);
}
console.log(`Gateway connected: ${gateways[0].name}`);

console.log('Provisioning done.');
