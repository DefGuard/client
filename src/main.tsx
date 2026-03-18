import { error } from '@tauri-apps/plugin-log';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from './components/App/App';
import { errorDetail } from './shared/utils/errorDetail';

// Forward uncaught JS errors to the Tauri backend log
window.onerror = (message, source, lineno, colno, err) => {
  const detail = err?.stack ?? `${message} (${source}:${lineno}:${colno})`;
  error(`[uncaught error] ${detail}`);
  // returning false lets the error propagate to the browser DevTools console
  return false;
};

// Forward unhandled promise rejections to the Tauri backend log
window.addEventListener('unhandledrejection', (event) => {
  error(`[unhandled rejection] ${errorDetail(event.reason)}`);
});

const rootElement = document.getElementById('root') as HTMLElement;

const root = createRoot(rootElement);

root.render(
  <StrictMode>
    <App />
  </StrictMode>,
);
