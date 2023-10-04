import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { attachConsole } from 'tauri-plugin-log-api';

import { App } from './components/App/App';
import { ToastManager } from './shared/defguard-ui/components/Layout/ToastManager/ToastManager';

// Attach console for logging
attachConsole();

const queryClient = new QueryClient();

const rootElement = document.getElementById('root') as HTMLElement;

const root = createRoot(rootElement);

root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      <ToastManager />
    </QueryClientProvider>
  </StrictMode>,
);
