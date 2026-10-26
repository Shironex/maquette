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
      // The Rust crates' own web files: the frame proxy's page bridge shim, for mock mode.
      '@crates': resolve(import.meta.dirname, '../../crates'),
    },
    dedupe: ['react', 'react-dom'],
  },
  // The Inbox and Inspector are lazy routes. Without this, Vite only finds
  // three and fiber when one first opens, re-bundles mid session and the page
  // ends up with two copies of React (an invalid hook call in the Canvas).
  optimizeDeps: {
    entries: ['index.html', 'src/**/*.tsx', '!src/**/*.test.tsx'],
    include: [
      'react',
      'react-dom',
      'react-dom/client',
      'three',
      '@react-three/fiber',
      'three/examples/jsm/controls/OrbitControls.js',
      'three/examples/jsm/libs/meshopt_decoder.module.js',
      'three/examples/jsm/loaders/GLTFLoader.js',
      'three/examples/jsm/loaders/KTX2Loader.js',
      'three/examples/jsm/utils/SkeletonUtils.js',
    ],
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
