import { useCallback } from 'react';

import { useToaster } from '../defguard-ui/hooks/toasts/useToaster';

export const useClipboard = () => {
  const toaster = useToaster();

  const writeToClipboard = useCallback(
    async (content: string, customMessage?: string) => {
      if (window.isSecureContext) {
        try {
          await navigator.clipboard.writeText(content);
          if (customMessage) {
            toaster.success(customMessage);
          } else {
            toaster.success('Content copied to clipboard');
          }
        } catch (e) {
          toaster.error('Writing to clipboard failed !');
          console.error(e);
        }
      } else {
        toaster.warning('Cannot access clipboard in insecure contexts');
      }
    },
    [toaster],
  );

  return {
    writeToClipboard,
  };
};
