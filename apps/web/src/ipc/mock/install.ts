/**
 * Mock mode: imported first by `main.tsx` in a plain browser during
 * development, so every generated command resolves against the in-memory
 * backend. Tauri builds never reach this file.
 */
import { mockIPC } from '@tauri-apps/api/mocks';
import { events } from '../generated/bindings';
import { fileUrl, registerMockFile } from '../files';
import { createBackend } from './backend';
import { DEFAULT_ROOT, HOME } from './fixtures';
import { PREVIEW_FILE, thumbsDir } from './inbox-fixtures';

const params = new URLSearchParams(window.location.search);
const root = DEFAULT_ROOT.replace('~', HOME);

// The Board and Inbox renders and the inbox models, bundled under assets/mock.
const renders = import.meta.glob<string>('../../assets/mock/*.webp', {
  eager: true,
  import: 'default',
});
const models = import.meta.glob<string>('../../assets/mock/models/*.glb', {
  eager: true,
  query: '?url',
  import: 'default',
});
const base = (path: string) => path.slice(path.lastIndexOf('/') + 1);
for (const [path, url] of Object.entries(renders)) {
  registerMockFile(`${root}/projects/moon-island/thumbs/${base(path)}`, url);
  registerMockFile(`${thumbsDir(`${root}/inbox`)}/${base(path)}`, url);
}
for (const [file, model] of Object.entries(PREVIEW_FILE)) {
  const url = models[`../../assets/mock/models/${model}`];
  if (url) registerMockFile(`${root}/inbox/${file}`, url);
}

const handle = createBackend({
  scenario: params.get('scenario') ?? 'first-run',
  probeDelay: Number(params.get('delay') ?? 700),
  pace: Number(params.get('pace') ?? 260),
  emit: event => {
    if (event.name === 'pipeline') void events.pipelineUpdate.emit(event.payload);
    else void events.inboxChanged.emit({ paths: [] });
  },
  files: {
    alias: (path, from) => {
      const url = fileUrl(from);
      if (url) registerMockFile(path, url);
    },
    store: (path, bytes) => {
      const copy = new Uint8Array(bytes);
      registerMockFile(path, URL.createObjectURL(new Blob([copy], { type: 'image/png' })));
    },
  },
});

mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
  shouldMockEvents: true,
});
