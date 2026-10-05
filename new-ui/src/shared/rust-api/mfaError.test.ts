import { describe, expect, it } from 'vitest';
import { isCancelled, mfaErrorMessage } from './mfaError';

describe('mfaErrorMessage', () => {
  it('uses the message the backend wrote', () => {
    expect(
      mfaErrorMessage('{"type":"mfa_rejected","message":"No security key detected"}'),
    ).toBe('No security key detected');
  });

  it('words errors that arrive as a bare type instead of showing JSON', () => {
    expect(mfaErrorMessage('{"type":"timeout"}')).toBe(
      'The operation timed out. Please try again.',
    );
    expect(mfaErrorMessage('{"type":"cancelled"}')).toBe('Authentication was cancelled.');
    expect(mfaErrorMessage('{"type":"something_new"}')).toBe(
      'Authentication failed. Please try again.',
    );
  });

  it('passes plain text through', () => {
    expect(mfaErrorMessage('PIN is required')).toBe('PIN is required');
  });
});

describe('isCancelled', () => {
  it('recognises only a cancellation', () => {
    expect(isCancelled('{"type":"cancelled"}')).toBe(true);
    expect(isCancelled('{"type":"timeout"}')).toBe(false);
    expect(isCancelled('cancelled')).toBe(false);
  });
});
