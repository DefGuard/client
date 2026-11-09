import './app/day.ts';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './app/App.tsx';
import './shared/scss/index.scss';
import { logUiCrash, UiCrashSource } from './shared/utils/logUiCrash.ts';

window.addEventListener('error', (event) => {
  logUiCrash(UiCrashSource.Window, event.error ?? event.message);
});

// biome-ignore lint/style/noNonNullAssertion: this element is static in index.html
createRoot(document.getElementById('root')!, {
  // replacing the defaults drops React's own console report, so keep it
  onCaughtError: (error, { componentStack }) => {
    console.error(error);
    logUiCrash(UiCrashSource.Caught, error, componentStack);
  },
  onUncaughtError: (error, { componentStack }) => {
    console.error(error);
    logUiCrash(UiCrashSource.Uncaught, error, componentStack);
  },
}).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
