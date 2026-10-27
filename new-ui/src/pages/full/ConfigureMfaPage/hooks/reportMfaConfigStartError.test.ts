import { describe, expect, it, vi } from 'vitest';
import { Snackbar } from '../../../../shared/providers/snackbar/snackbar';
import { reportMfaConfigStartError } from './reportMfaConfigStartError';

vi.mock('@tauri-apps/plugin-log', () => ({ error: vi.fn() }));
vi.mock('../../../../shared/providers/snackbar/snackbar', () => ({
  Snackbar: { error: vi.fn() },
}));
vi.mock('../components/EdgeComsError/useEdgeComsErrorStore', () => ({
  showEdgeComsError: vi.fn(),
}));

describe('reportMfaConfigStartError', () => {
  it('points at the profile when the account MFA cannot be used here', () => {
    reportMfaConfigStartError(JSON.stringify({ type: 'mfa_not_usable' }));

    expect(Snackbar.error).toHaveBeenCalledWith(
      expect.stringContaining('in your Defguard profile'),
    );
  });
});
