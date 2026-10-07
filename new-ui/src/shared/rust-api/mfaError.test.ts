import { describe, expect, it } from 'vitest';
import { isInvalidCode } from './mfaError';

describe('isInvalidCode', () => {
  it('matches exact rejected-code messages without case sensitivity', () => {
    expect(isInvalidCode('Unauthorized')).toBe(true);
    expect(isInvalidCode(' unauthorized ')).toBe(true);
    expect(isInvalidCode('invalid code')).toBe(true);
    expect(isInvalidCode(' Invalid Code ')).toBe(true);
  });

  it('does not treat rejection details as retryable code messages', () => {
    expect(isInvalidCode('invalid code: session expired')).toBe(false);
    expect(isInvalidCode('invalid token')).toBe(false);
  });
});
