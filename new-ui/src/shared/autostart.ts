import { invoke } from '@tauri-apps/api/core';

export type AutostartStatus = 'enabled' | 'disabled' | 'requiresApproval' | 'notFound';

/** Read the OS login registration; no autostart preference is stored in AppConfig. */
export const getAutostartStatus = (): Promise<AutostartStatus> =>
  invoke<AutostartStatus>('get_autostart_status');

/** Update the OS login registration and return its effective status. */
export const setAutostartEnabled = (enabled: boolean): Promise<AutostartStatus> =>
  invoke<AutostartStatus>('set_autostart_enabled', { enabled });
