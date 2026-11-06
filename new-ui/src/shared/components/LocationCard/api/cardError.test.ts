import { describe, expect, it, vi } from 'vitest';
import { MfaFailureText } from '../../../rust-api/mfaError';
import type { LocationInfo } from '../../../rust-api/types';
import { IconKind } from '../../Icon';
import { connectErrorToCard, MfaRetry, mfaErrorToCard } from './cardError';
import { DEFAULT_CONNECTION_ERROR } from './connectError';

vi.mock('./connectError', () => ({
  DEFAULT_CONNECTION_ERROR: 'Service unavailable',
}));

const mfaError = (type: string, message?: string) => JSON.stringify({ type, message });

describe('mfaErrorToCard', () => {
  it.each([
    [
      'service unavailable',
      mfaError('network_error', 'unreachable'),
      DEFAULT_CONNECTION_ERROR,
      IconKind.ServiceUnavailable,
      MfaRetry.ResumeStep,
    ],
    [
      'timeout',
      mfaError('timeout'),
      MfaFailureText.Timeout,
      IconKind.WarningOutlined,
      MfaRetry.ResumeStep,
    ],
    [
      'outdated mobile app',
      mfaError('mobile_client_outdated'),
      'Your Defguard mobile app is out of date.\nUpgrade to approve this connection.',
      IconKind.DisableMfa,
      null,
    ],
    [
      'attempt limit',
      mfaError('attempt_limit', 'Too many failed MFA attempts'),
      'Too many failed MFA attempts',
      IconKind.WarningOutlined,
      null,
    ],
    [
      'mobile rejection',
      mfaError('mfa_rejected', 'Mobile approval rejected'),
      'Mobile approval rejected',
      IconKind.WarningOutlined,
      null,
    ],
    [
      'blocked security key',
      mfaError(
        'mfa_rejected',
        'Security key rejected the PIN: the security key is locked after too many PIN attempts',
      ),
      'Security key rejected the PIN: the security key is locked after too many PIN attempts',
      IconKind.WarningOutlined,
      null,
    ],
    [
      'stale attempt',
      mfaError('mfa_rejected', 'stale MFA attempt'),
      MfaFailureText.StaleAttempt,
      IconKind.WarningOutlined,
      MfaRetry.RestartMfa,
    ],
    [
      'session expired',
      mfaError('other', 'login session not found'),
      MfaFailureText.SessionExpired,
      IconKind.WarningOutlined,
      MfaRetry.RestartMfa,
    ],
    [
      'VPN failure after MFA',
      'VPN connection failed: interface error',
      MfaFailureText.ConnectFailure,
      IconKind.WarningOutlined,
      MfaRetry.RestartMfa,
    ],
    [
      'worded server error',
      mfaError('mfa_rejected', 'No security key detected'),
      'No security key detected',
      IconKind.WarningOutlined,
      MfaRetry.RestartMfa,
    ],
    [
      'unstructured error',
      'Instance not found',
      MfaFailureText.Unknown,
      IconKind.WarningOutlined,
      MfaRetry.RestartMfa,
    ],
  ])('maps %s', (_, err, message, icon, retry) => {
    expect(mfaErrorToCard(err)).toEqual({ message, icon, retry });
  });
});

describe('connectErrorToCard', () => {
  const location = { name: 'Office' } as LocationInfo;

  it('uses the service error for an unavailable service', () => {
    expect(
      connectErrorToCard(
        { kind: 'serviceUnavailable', message: 'down', conflicts: [] },
        location,
      ),
    ).toEqual({ message: DEFAULT_CONNECTION_ERROR, icon: IconKind.ServiceUnavailable });
  });

  it.each([
    ['other', { kind: 'other' as const, message: 'daemon error', conflicts: [] }],
    ['unparseable', null],
  ])('names the location for an %s error', (_, connectError) => {
    expect(connectErrorToCard(connectError, location)).toEqual({
      message: 'Failed to connect to Office. Please try again.',
      icon: IconKind.WarningOutlined,
    });
  });
});
