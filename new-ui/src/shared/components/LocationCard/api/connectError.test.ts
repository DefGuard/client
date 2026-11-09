import { describe, expect, it, vi } from 'vitest';
import { ConnectionType, type LocationInfo } from '../../../rust-api/types';
import { preflightConnect } from './connectError';

const mocks = vi.hoisted(() => ({
  checkCanConnect: vi.fn(),
  showSystemExtensionModal: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({ emitTo: vi.fn() }));
vi.mock('./systemExtension', () => ({
  openSystemExtensionModal: vi.fn(),
  showSystemExtensionModal: mocks.showSystemExtensionModal,
}));
vi.mock('../../../rust-api/api', () => ({
  api: { checkCanConnect: mocks.checkCanConnect },
}));

const location = { id: 7 } as LocationInfo;
const conflict = { id: 3, connection_type: ConnectionType.Location, name: 'Office' };

describe('preflightConnect', () => {
  it('returns the conflicts only for a route conflict error', async () => {
    mocks.checkCanConnect.mockRejectedValueOnce({
      kind: 'routeConflict',
      message: 'conflict',
      conflicts: [conflict],
    });
    await expect(preflightConnect(location)).resolves.toEqual([conflict]);
    expect(mocks.checkCanConnect).toHaveBeenCalledWith(7);

    mocks.checkCanConnect.mockResolvedValueOnce(undefined);
    await expect(preflightConnect(location)).resolves.toEqual([]);

    mocks.checkCanConnect.mockRejectedValueOnce({ kind: 'other', message: 'db' });
    await expect(preflightConnect(location)).resolves.toEqual([]);
  });

  it('shows the system extension modal and blocks when the extension is disabled', async () => {
    mocks.checkCanConnect.mockRejectedValueOnce({
      kind: 'systemExtensionDisabled',
      message: 'disabled',
    });
    await expect(preflightConnect(location)).resolves.toBeNull();
    expect(mocks.showSystemExtensionModal).toHaveBeenCalledOnce();
  });
});
