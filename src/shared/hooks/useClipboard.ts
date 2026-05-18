import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { error } from '@tauri-apps/plugin-log';
import { useCallback } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { useToaster } from '../defguard-ui/hooks/toasts/useToaster';
import { errorDetail } from '../utils/errorDetail';

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
        const detail = errorDetail(e);
        error(`Failed to write to clipboard: ${detail}`);
      }
    },
    [LL.common.messages, toaster],
  );

  return {
    writeToClipboard,
  };
};
