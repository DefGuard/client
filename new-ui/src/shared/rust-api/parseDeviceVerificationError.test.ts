import { describe, expect, it } from 'vitest';
import { parseDeviceVerificationError } from './parseDeviceVerificationError';

describe('parseDeviceVerificationError', () => {
  it('parses tagged errors', () => {
    expect(parseDeviceVerificationError('{"type":"device_blocked"}')).toEqual({
      type: 'device_blocked',
    });
    expect(
      parseDeviceVerificationError(
        '{"type":"proxy","status":502,"message":"core timeout"}',
      ),
    ).toEqual({ type: 'proxy', status: 502, message: 'core timeout' });
  });

  it('returns null for anything else', () => {
    expect(parseDeviceVerificationError('Instance not found')).toBeNull();
    expect(parseDeviceVerificationError('{"message":"no type"}')).toBeNull();
    expect(parseDeviceVerificationError(undefined)).toBeNull();
  });
});
