import { useNavigate } from '@tanstack/react-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect } from 'react';
import { WindowId } from '../consts';

const isFullView = getCurrentWindow().label === WindowId.FullView;

export const usePlaygroundShortcut = () => {
  const navigate = useNavigate();

  useEffect(() => {
    if (!import.meta.env.DEV || !isFullView) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'p') {
        event.preventDefault();
        void navigate({ to: '/playground' });
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [navigate]);
};
