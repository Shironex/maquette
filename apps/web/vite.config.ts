import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'node:path';
import { singleBasisTranscoder } from './vite/basis-transcoder.ts';

// The Tauri shell points `devUrl` here; keep the two in step.
const PORT = 15190;

export default defineConfig({
  plugins: [singleBasisTranscoder(), tailwindcss(), react()],
  base: './',
  clearScreen: false,
  resolve: {
    alias: {
      '@': resolve(import.meta.dirname, './src'),
    },
  },
  server: {
    port: PORT,
    strictPort: true,
    fs: {
      allow: ['../..'],
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    target: 'es2022',
  },
});
