import { emitTo } from '@tauri-apps/api/event';
import { WindowId } from '../consts';
import { api } from '../rust-api/api';
import type { TauriEventValue } from '../rust-api/types';

// The compact window is too small for modals, so the full view is brought forward to show them.
export const emitInFullView = async (event: TauriEventValue, payload?: unknown) => {
  await api.swapToFullView();
  await emitTo(WindowId.FullView, event, payload);
};
