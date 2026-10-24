import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    name: 'ui',
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
  },
});
