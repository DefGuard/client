import { z } from 'zod';
import {
  patternValidEndpoint,
  patternValidIpV6WithPort,
  patternValidWireguardKey,
} from './patterns';

export const createZodIssue = (
  message: string,
  path: PropertyKey[],
): z.core.$ZodIssueCustom => ({
  code: 'custom',
  message,
  path,
});

// Shared field schemas for the WireGuard tunnel forms (add/edit). Kept here so
// the tunnel wizard and the edit-tunnel modal validate identically.

// WireGuard endpoint (host:port): IPv4:port, domain[:port], or [IPv6]:port.
export const endpointSchema = z
  .string()
  .refine(
    (v) => patternValidEndpoint.test(v) || patternValidIpV6WithPort.test(v),
    'Invalid address',
  );

// A required WireGuard key.
export const wireguardKeySchema = z
  .string()
  .refine((v) => patternValidWireguardKey.test(v), 'Invalid WireGuard key');

// An optional WireGuard key - an empty value is allowed (e.g. preshared key).
export const optionalWireguardKeySchema = z
  .string()
  .refine((v) => !v || patternValidWireguardKey.test(v), 'Invalid WireGuard key');

const ipOrCidrSchema = z.union([z.ipv4(), z.ipv6(), z.cidrv4(), z.cidrv6()]);

const isValidIpList = (value: string) =>
  value
    .split(',')
    .map((ip) => ip.trim())
    .every((ip) => ipOrCidrSchema.safeParse(ip).success);

// A required comma-separated list of interface addresses or CIDR ranges.
export const interfaceAddressesSchema = z
  .string()
  .refine((value) => Boolean(value) && isValidIpList(value), 'Field is invalid');

// Comma-separated list of allowed IP addresses or CIDR ranges; an empty value is allowed.
export const allowedIpsSchema = z
  .string()
  .refine(
    (value) => !value || isValidIpList(value),
    'Invalid IP address or CIDR notation',
  );
