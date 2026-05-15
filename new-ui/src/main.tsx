import './app/day.ts';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './app/App.tsx';
import './shared/scss/index.scss';

// biome-ignore lint/style/noNonNullAssertion: this element is static in index.html
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
