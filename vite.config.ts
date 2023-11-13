import react from '@vitejs/plugin-react-swc';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import { defineConfig } from 'vite';

// https://vitejs.dev/config/
export default defineConfig({
  base: './',
  plugins: [react()],
  clearScreen: false,
  server: {
    strictPort: true,
    port: 3000,
  },
  resolve: {
    alias: {
      '@scssutils': path.resolve('./src/shared/defguard-ui/scss/helpers'),
    },
  },
  css: {
    postcss: {
      plugins: [autoprefixer],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    chunkSizeWarningLimit: 10000000,
  },
});
