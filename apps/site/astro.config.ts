import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import tailwindcss from '@tailwindcss/vite';
import { basisTranscoder } from './vite/basis-transcoder.ts';

// Next to the web app's 15190 and its worktree ports; preview uses the same.
const PORT = 15292;

export default defineConfig({
  output: 'static',
  integrations: [react()],
  server: { port: PORT },
  build: { inlineStylesheets: 'auto' },
  vite: {
    plugins: [tailwindcss(), ...basisTranscoder()],
    server: { fs: { allow: ['../..'] } },
    build: { target: 'es2022' },
  },
});
