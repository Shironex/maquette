/**
 * Mock mode: imported first by `main.tsx` in a plain browser during
 * development, so every generated command resolves against the in-memory
 * backend. Tauri builds never reach this file.
 */
import { mockIPC } from '@tauri-apps/api/mocks';
import { events } from '../generated/bindings';
import { fileUrl, registerMockFile } from '../files';
import { createBackend } from './backend';
import { DEFAULT_ROOT, HOME, MOCK_PROJECT_IDS } from './fixtures';
import { PREVIEW_FILE, thumbsDir } from './inbox-fixtures';
import { conceptFiles, OBJECT_REF_FILES, REF_FILES } from './lab-fixtures';
import { INSPECT_FILES } from './inspect-fixtures';
import { LAYOUT_FILES } from './layout';
import { motionFiles } from './create-motion-handlers';
import { PROVIDER_REF_FILES } from './providers';
import { MOCK_SITE_URL, conceptPath, pageOption, posterPath } from './create-page-handlers';
import { MANY_STOPS, mockConceptPage, mockSitePage } from './page-site';
import { libraryFiles } from './library-fixtures';
import bridgeShim from '@crates/maquette-core/src/site/preview/frame/shim.js?raw';

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
for (const [file, source] of Object.entries({ ...REF_FILES, ...OBJECT_REF_FILES })) {
  const url = source.startsWith('concepts/')
    ? conceptUrl(source.slice('concepts/'.length))
    : renders[`../../assets/mock/${source}`];
  if (url) registerMockFile(`${projectDir}/${file}`, url);
}
for (const [file, source] of Object.entries(PROVIDER_REF_FILES)) {
  const url = renders[`../../assets/mock/${source}`];
  if (url) registerMockFile(`${projectDir}/${file}`, url);
}
for (const [path, url] of Object.entries(renders)) {
  for (const id of MOCK_PROJECT_IDS)
    registerMockFile(`${root}/projects/${id}/thumbs/${base(path)}`, url);
  registerMockFile(`${thumbsDir(`${root}/inbox`)}/${base(path)}`, url);
}
for (const [file, model] of Object.entries(PREVIEW_FILE)) {
  const url = models[`../../assets/mock/models/${model}`];
  if (url) registerMockFile(`${root}/inbox/${file}`, url);
}
for (const [file, render] of Object.entries(libraryFiles())) {
  const url = renders[`../../assets/mock/${render}`];
  if (url) registerMockFile(`${root}/library/${file}`, url);
}
// `?timeline=` brings the motion demo models along.
const motion = params.get('motion') ?? (params.get('timeline') ? 'demo' : null);
for (const [file, model] of Object.entries({
  ...INSPECT_FILES,
  ...LAYOUT_FILES,
  ...motionFiles(motion),
})) {
  const url = models[`../../assets/mock/models/${model}`];
  if (url) registerMockFile(`${root}/projects/moon-island/${file}`, url);
}
// `?motion=still` puts its prop in Rooftop cinema.
for (const [file, model] of Object.entries(motion === 'still' ? motionFiles(motion) : {})) {
  const url = models[`../../assets/mock/models/${model}`];
  if (url) registerMockFile(`${root}/projects/rooftop-cinema/${file}`, url);
}

// The Page view's dev server and page concept, as pages of their own.
const absolute = (url: string | undefined) => new URL(url ?? '', window.location.href).href;
const htmlUrl = (html: string) => URL.createObjectURL(new Blob([html], { type: 'text/html' }));
const stopPictures = [
  conceptUrl('moon-island-a.webp'),
  renders['../../assets/mock/sakura-large.webp'],
  renders['../../assets/mock/flower-shrubs.webp'],
  conceptUrl('moon-island-b.webp'),
  renders['../../assets/mock/island.webp'],
].map(absolute);
const tour = params.get('page') === 'stops-10' ? MANY_STOPS : undefined;
// `?page=shim`: a site without the bridge, carrying the shim the frame proxy adds.
const shim = params.get('page') === 'shim' ? bridgeShim : undefined;
registerMockFile(MOCK_SITE_URL, htmlUrl(mockSitePage({ stops: stopPictures }, tour, shim)));
registerMockFile(conceptPath(projectDir), htmlUrl(mockConceptPage(stopPictures[0] ?? '')));
stopPictures.forEach((url, i) => registerMockFile(posterPath(projectDir, i), url));

const handle = createBackend({
  scenario: params.get('scenario') ?? 'first-run',
  probeDelay: Number(params.get('delay') ?? 700),
  pace: Number(params.get('pace') ?? 260),
  lab: params.get('lab'),
  conceptMs: Number(params.get('conceptMs') ?? 4000),
  agents: params.get('agents'),
  projects: params.get('projects'),
  handoff: params.get('handoff'),
  inbox: params.get('inbox'),
  arrival: params.get('arrival'),
  built: params.get('built'),
  motion,
  page: pageOption(params),
  timeline: params.get('timeline'),
  game: params.get('game'),
  library: params.get('library'),
  // `?timeline=demo` lights the demo paths too, for its paths lane.
  paths: params.get('paths') ?? (params.get('timeline') === 'demo' ? 'demo' : null),
  emit: event => {
    if (event.name === 'page') void events.pageServerChanged.emit(event.payload);
    else if (event.name === 'built') {
      if (event.manifest) void events.projectChanged.emit({ project: event.manifest });
      void events.builtChanged.emit({ project: event.project });
    } else if (event.name === 'pipeline') void events.pipelineUpdate.emit(event.payload);
    else if (event.name === 'concept') void events.conceptUpdate.emit(event.payload);
    else if (event.name === 'mcp') void events.mcpUpdate.emit(event.payload);
    else if (event.name === 'downloads') void events.downloadsArrival.emit(event.payload);
    else if (event.name === 'provider') {
      const payload = event.payload;
      if (payload.type === 'project') void events.projectChanged.emit({ project: payload.project });
      else void events.providerUpdate.emit(payload);
    } else void events.inboxChanged.emit({ paths: [] });
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
