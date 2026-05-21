import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { MainBackground } from '../shared/components/MainBackground/MainBackground';
import { queryClient } from './query';
import { router } from './router';

function App() {
  return (
    <div id="app">
      <MainBackground />
      <div id="app-content">
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </div>
    </div>
  );
}

export default App;
