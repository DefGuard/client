import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { type as getOsType } from '@tauri-apps/plugin-os';
import { useEffect } from 'react';
import { MainBackground } from '../shared/components/MainBackground/MainBackground';
import { WindowDecorations } from '../shared/components/WindowDecorations/WindowDecorations';
import { queryClient } from './query';
import { router } from './router';

const isLinux = getOsType() === 'linux';

// WebKitGTK on Linux doesn't recompute 100dvh on window resize, so on Linux
// only we track window.innerHeight in JS instead. Windows/macOS keep the
// native 100dvh default (--app-dvh's fallback value in _shared_tokens.scss) unchanged.
function applyLinuxViewportHeightFix(): () => void {
  const update = () => {
    document.documentElement.style.setProperty('--app-dvh', `${window.innerHeight}px`);
  };
  update();
  window.addEventListener('resize', update);
  return () => window.removeEventListener('resize', update);
}

function App() {
  useEffect(() => {
    if (!isLinux) return;
    return applyLinuxViewportHeightFix();
  }, []);

  return (
    <div id="app">
      <MainBackground />
      <WindowDecorations />
      <div id="app-content">
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </div>
    </div>
  );
}

export default App;
