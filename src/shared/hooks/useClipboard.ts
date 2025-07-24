import { useCallback } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { useToaster } from '../defguard-ui/hooks/toasts/useToaster';

export const useClipboard = () => {
  const { LL } = useI18nContext();

  const toaster = useToaster();

  const writeToClipboard = useCallback(
    async (content: string, customMessage?: string) => {
      if (window.isSecureContext) {
        try {
          await navigator.clipboard.writeText(content);
          if (customMessage) {
            toaster.success(customMessage);
          } else {
            toaster.success(LL.common.messages.clipboard.success());
          }
        } catch (e) {
          toaster.error(LL.common.messages.clipboard.error());
          console.error(e);
        }
      } else {
        toaster.warning(LL.common.messages.insecureContext());
      }
    },
    [LL.common.messages, toaster],
  );

  return {
    writeToClipboard,
  };
};
