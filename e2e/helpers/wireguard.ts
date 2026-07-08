import { generateKeyPairSync } from 'node:crypto';
import type { CoreApi } from './coreApi.js';

export const generateWireguardKeys = () => {
  const { privateKey, publicKey } = generateKeyPairSync('x25519');
  return {
    privateKey: privateKey
      .export({ type: 'pkcs8', format: 'der' })
      .subarray(-32)
      .toString('base64'),
    publicKey: publicKey
      .export({ type: 'spki', format: 'der' })
      .subarray(-32)
      .toString('base64'),
  };
};

export type TunnelConfig = {
  name: string;
  prvkey: string;
  pubkey: string;
  address: string;
  serverPubkey: string;
  allowedIps: string;
  endpoint: string;
  dns: string;
  keepalive: string;
};

export const provisionTunnel = async (
  core: CoreApi,
  networkId: number,
  name: string,
): Promise<TunnelConfig> => {
  const keys = generateWireguardKeys();
  const ip = await core.findAvailableIp(networkId);
  const conf = await core.addNetworkDevice(networkId, name, [ip], keys.publicKey);
  const field = (re: RegExp) => conf.match(re)?.[1]?.trim() ?? '';
  return {
    name,
    prvkey: keys.privateKey,
    pubkey: keys.publicKey,
    address: field(/Address\s*=\s*(.+)/),
    serverPubkey: field(/PublicKey\s*=\s*(.+)/),
    allowedIps: field(/AllowedIPs\s*=\s*(.+)/),
    endpoint: field(/Endpoint\s*=\s*(.+)/),
    dns: field(/DNS\s*=\s*(.+)/),
    keepalive: field(/PersistentKeepalive\s*=\s*(.+)/) || '25',
  };
};
