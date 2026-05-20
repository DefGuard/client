import react from '@vitejs/plugin-react-swc';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async ({ command }) => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    strictPort: true,
    port: 5071,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
  resolve: {
    alias: {
      '@scssutils': path.resolve('./src/shared/defguard-ui/scss/helpers'),
    },
  },
  css: {
    preprocessorOptions: {
      scss: {
        additionalData: `@use "@scssutils" as *;\n`,
      },
    },
    postcss: {
      plugins: [autoprefixer],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  base: command === 'build' ? '/old-ui/' : './',
  build: {
    chunkSizeWarningLimit: 10000000,
    outDir: './dist/old-ui',
    emptyOutDir: true,
  },
}));
