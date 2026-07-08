import { $, browser } from '@wdio/globals';
import { switchToFullView } from './windows.js';

type TauriWindow = {
  __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
};

export const resetInstances = async () => {
  await switchToFullView();
  await $('a[href="/full/add"]').click();
  await $('#add-page-view').waitForDisplayed();
  await browser.execute(async () => {
    const { invoke } = (window as unknown as TauriWindow).__TAURI_INTERNALS__;
    const instances = (await invoke('all_instances')) as Array<{ id: number }>;
    await Promise.all(
      instances.map((instance) => invoke('delete_instance', { instanceId: instance.id })),
    );
  });
};
