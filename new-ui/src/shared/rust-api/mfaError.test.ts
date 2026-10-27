import { describe, expect, it } from 'vitest';
import { isInvalidCode, mfaErrorMessage } from './mfaError';

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

describe('mfaErrorMessage', () => {
  it('maps the outdated mobile client type to an upgrade message', () => {
    expect(mfaErrorMessage(JSON.stringify({ type: 'mobile_client_outdated' }))).toBe(
      'Your Defguard mobile app is out of date.\nUpgrade to approve this connection.',
    );
  });

  it('preserves other error messages and plain text', () => {
    expect(
      mfaErrorMessage(JSON.stringify({ type: 'other', message: 'Existing error' })),
    ).toBe('Existing error');
    expect(mfaErrorMessage('Plain error')).toBe('Plain error');
  });
});
