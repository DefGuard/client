import { describe, expect, it, vi } from 'vitest';
import { ConnectionType, type LocationInfo } from '../../../rust-api/types';
import { findRouteConflicts } from './connectError';

const mocks = vi.hoisted(() => ({ checkRouteConflict: vi.fn() }));

vi.mock('@tauri-apps/api/event', () => ({ emitTo: vi.fn() }));
vi.mock('../../../rust-api/api', () => ({
  api: { checkRouteConflict: mocks.checkRouteConflict },
}));

const location = { id: 7 } as LocationInfo;
const conflict = { id: 3, connection_type: ConnectionType.Location, name: 'Office' };

describe('findRouteConflicts', () => {
  it('returns the conflicts only for a route conflict error', async () => {
    mocks.checkRouteConflict.mockRejectedValueOnce({
      kind: 'routeConflict',
      message: 'conflict',
      conflicts: [conflict],
    });
    await expect(findRouteConflicts(location)).resolves.toEqual([conflict]);
    expect(mocks.checkRouteConflict).toHaveBeenCalledWith(7);

    mocks.checkRouteConflict.mockResolvedValueOnce(undefined);
    await expect(findRouteConflicts(location)).resolves.toEqual([]);

    mocks.checkRouteConflict.mockRejectedValueOnce({ kind: 'other', message: 'db' });
    await expect(findRouteConflicts(location)).resolves.toEqual([]);
  });
});
