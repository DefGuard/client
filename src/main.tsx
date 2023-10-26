import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from './components/App/App';

const rootElement = document.getElementById('root') as HTMLElement;

const root = createRoot(rootElement);

root.render(
  <StrictMode>
    <App />
  </StrictMode>,
);
