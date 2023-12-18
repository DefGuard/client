import { ReactNode, useEffect } from 'react';

import { useClientStore } from '../../../pages/client/hooks/useClientStore';
import { ThemeKey } from '../../defguard-ui/hooks/theme/types';

type Props = {
  children: ReactNode;
};
// this sync settings theme with html dataset
export const ThemeProvider = ({ children }: Props) => {
  const currentTheme = useClientStore((state) => state.settings.theme);

  useEffect(() => {
    const current = document.documentElement.dataset.theme as ThemeKey;
    if (currentTheme != current) {
      document.documentElement.dataset.theme = currentTheme;
    }
  }, [currentTheme]);

  return <>{children}</>;
};
