import react from '@vitejs/plugin-react';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import { defineConfig } from 'vite';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import { devtools } from '@tanstack/devtools-vite';

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async ({ command }) => ({
  plugins: [devtools(), tanstackRouter(), react()],
  clearScreen: false,
  server: {
    strictPort: true,
    port: 5072,
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
      '@scssutils': path.resolve('./src/shared/scss/global'),
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
  base: '/',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
  },
}));
