import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useCallback } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { useToaster } from '../defguard-ui/hooks/toasts/useToaster';

export const useClipboard = () => {
  const { LL } = useI18nContext();

  const toaster = useToaster();

  const writeToClipboard = useCallback(
    async (content: string, customMessage?: string) => {
      try {
        await writeText(content);
        if (customMessage) {
          toaster.success(customMessage);
        } else {
          toaster.success(LL.common.messages.clipboard.success());
        }
      } catch (e) {
        toaster.error(LL.common.messages.clipboard.error());
        console.error(e);
      }
    },
    [LL.common.messages, toaster],
  );

  return {
    writeToClipboard,
  };
};
