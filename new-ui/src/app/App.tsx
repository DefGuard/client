import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { MainBackground } from '../shared/components/MainBackground/MainBackground';
import { WindowDecorations } from '../shared/components/WindowDecorations/WindowDecorations';
import { queryClient } from './query';
import { router } from './router';

function App() {
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
