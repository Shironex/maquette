/**
 * Mock mode: imported first by `main.tsx` in a plain browser during
 * development, so every generated command resolves against the in-memory
 * backend. Tauri builds never reach this file.
 */
import { mockIPC } from '@tauri-apps/api/mocks';
import { createBackend } from './backend';

const params = new URLSearchParams(window.location.search);
const handle = createBackend({
  scenario: params.get('scenario') ?? 'first-run',
  probeDelay: Number(params.get('delay') ?? 700),
});

mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>));
