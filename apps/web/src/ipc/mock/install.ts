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
import { conceptFiles, REF_FILES } from './lab-fixtures';
import { INSPECT_FILES } from './inspect-fixtures';

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
const concepts = import.meta.glob<string>('../../assets/mock/concepts/*.webp', {
  eager: true,
  import: 'default',
});
const base = (path: string) => path.slice(path.lastIndexOf('/') + 1);
const projectDir = `${root}/projects/moon-island`;
const conceptUrl = (name: string) => concepts[`../../assets/mock/concepts/${name}`];
for (const [file, name] of Object.entries(conceptFiles())) {
  const url = conceptUrl(name);
  if (url) registerMockFile(`${projectDir}/${file}`, url);
}
for (const path of Object.keys(concepts)) {
  registerMockFile(`${projectDir}/concepts/__pool/${base(path)}`, concepts[path] ?? '');
}
for (const [file, source] of Object.entries(REF_FILES)) {
  const url = source.startsWith('concepts/')
    ? conceptUrl(source.slice('concepts/'.length))
    : renders[`../../assets/mock/${source}`];
  if (url) registerMockFile(`${projectDir}/${file}`, url);
}
for (const [path, url] of Object.entries(renders)) {
  registerMockFile(`${root}/projects/moon-island/thumbs/${base(path)}`, url);
  registerMockFile(`${thumbsDir(`${root}/inbox`)}/${base(path)}`, url);
}
for (const [file, model] of Object.entries(PREVIEW_FILE)) {
  const url = models[`../../assets/mock/models/${model}`];
  if (url) registerMockFile(`${root}/inbox/${file}`, url);
}
for (const [file, model] of Object.entries(INSPECT_FILES)) {
  const url = models[`../../assets/mock/models/${model}`];
  if (url) registerMockFile(`${root}/projects/moon-island/${file}`, url);
}

const handle = createBackend({
  scenario: params.get('scenario') ?? 'first-run',
  probeDelay: Number(params.get('delay') ?? 700),
  pace: Number(params.get('pace') ?? 260),
  lab: params.get('lab'),
  conceptMs: Number(params.get('conceptMs') ?? 4000),
  emit: event => {
    if (event.name === 'pipeline') void events.pipelineUpdate.emit(event.payload);
    else if (event.name === 'concept') void events.conceptUpdate.emit(event.payload);
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
    sheet: async (path, sources) => {
      const url = await contactSheet(sources.map(s => fileUrl(s)).filter(Boolean) as string[]);
      if (url) registerMockFile(path, url);
    },
  },
});

mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
  shouldMockEvents: true,
});

/** Tiles images the way the ffmpeg contact sheet does: 640 x 360 cells, 12 px gaps. */
async function contactSheet(urls: string[]): Promise<string | null> {
  const images = await Promise.all(
    urls.map(
      url =>
        new Promise<HTMLImageElement | null>(resolve => {
          const img = new Image();
          img.onload = () => resolve(img);
          img.onerror = () => resolve(null);
          img.src = url;
        })
    )
  );
  const loaded = images.filter((i): i is HTMLImageElement => i !== null);
  if (loaded.length === 0) return null;
  const cols = loaded.length <= 1 ? 1 : loaded.length <= 4 ? 2 : loaded.length <= 9 ? 3 : 4;
  const rows = Math.ceil(loaded.length / cols);
  const [w, h, gap] = [640, 360, 12];
  const canvas = document.createElement('canvas');
  canvas.width = gap + cols * (w + gap);
  canvas.height = gap + rows * (h + gap);
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  ctx.fillStyle = '#1b1916';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  loaded.forEach((img, i) => {
    const scale = Math.min(w / img.width, h / img.height);
    const [dw, dh] = [img.width * scale, img.height * scale];
    const x = gap + (i % cols) * (w + gap) + (w - dw) / 2;
    const y = gap + Math.floor(i / cols) * (h + gap) + (h - dh) / 2;
    ctx.drawImage(img, x, y, dw, dh);
  });
  const blob = await new Promise<Blob | null>(resolve => canvas.toBlob(resolve, 'image/png'));
  return blob ? URL.createObjectURL(blob) : null;
}
