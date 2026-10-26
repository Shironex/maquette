/**
 * The Page view in mock mode. The "dev server" is a page the mock serves
 * (see `page-site.ts`); starting it takes a moment, as a real one does.
 *
 * `?page=` holds a state for screenshots: `starting` stays starting,
 * `failed` fails as a site without its packages does, `no-site` takes
 * Moon Island's site repo away, `stops-10` gives the page ten stops.
 * `framed` (with `&site=` and `&frame=`) runs at a real dev server that
 * forbids framing, behind a real frame proxy, both started by the visual
 * check. Any `?page=` also puts the last ship on the site, so the Page view
 * has a current ship to compare this run with.
 */
import type {
  DevCommand,
  InspectModel,
  InspectView,
  PageSettings,
  PageView,
  ProjectManifest,
  ServerStatus,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';

/** Where the mock dev server answers. */
export const MOCK_SITE_URL = 'http://localhost:4321/';

/** `?page=`, with the framed state's two addresses folded in: `framed <site> <frame>`. */
export function pageOption(params: URLSearchParams): string | null {
  const page = params.get('page');
  if (page !== 'framed') return page;
  return `framed ${params.get('site') ?? MOCK_SITE_URL} ${params.get('frame') ?? ''}`;
}

/** The dev server and the frame proxy in front of it, for the state. */
function addresses(state: string | null): { url: string; frameUrl: string | null } {
  const [kind, site, frame] = (state ?? '').split(' ');
  if (kind !== 'framed' || !site) return { url: MOCK_SITE_URL, frameUrl: null };
  return { url: site, frameUrl: frame || null };
}

/** The HTML page concept the mock file picker returns, inside the workspace. */
export const conceptPath = (projectDir: string) => `${projectDir}/concepts/home_v2.html`;

const IDLE: ServerStatus = {
  state: 'idle',
  project: null,
  site: null,
  url: null,
  frameUrl: null,
  command: null,
  owner: null,
  external: false,
  startedAt: null,
  message: null,
  log: [],
};

const DEFAULTS: PageSettings = {
  version: 1,
  devCommand: null,
  concept: null,
  source: 'site',
  path: '/',
};

interface PageContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  projectDir: (id: string) => string;
  emit: (status: ServerStatus) => void;
  state: string | null;
  /** Milliseconds the server takes to answer. */
  startMs: number;
}

function detected(settings: PageSettings): DevCommand {
  if (settings.devCommand) {
    return {
      argv: settings.devCommand.split(/\s+/),
      source: 'settings',
      url: null,
      manager: null,
      script: null,
      lockfile: null,
    };
  }
  return {
    argv: ['pnpm', 'run', 'dev', '--port', '{port}'],
    source: 'package-json',
    url: null,
    manager: 'pnpm',
    script: 'dev',
    lockfile: 'pnpm-lock.yaml',
  };
}

function startLog(command: string): string[] {
  return [
    `$ ${command}`,
    '',
    '> moon-island-site@0.3.0 dev',
    '> astro dev --port 4321',
    '',
    '  astro  v5.14.1 ready in 812 ms',
  ];
}

/** Where this run's stop posters are, as the Posters step writes them. */
export const posterPath = (projectDir: string, stop: number) =>
  `${projectDir}/web/posters/stop${stop}.webp`;

/**
 * The site's last ship (older builds of the same models, before the
 * character) and this run's stop posters.
 */
export function withShip(view: InspectView, siteRepo: string, projectDir: string): InspectView {
  const older: Record<string, number> = { sakura: 1.27, flower_shrubs: 1.18 };
  const site: InspectModel[] = view.models
    .filter(m => m.source === 'shipped' && older[m.model] !== undefined)
    .map(m => {
      const k = older[m.model] ?? 1;
      return {
        ...m,
        id: `site/${m.model}`,
        source: 'site',
        file: `${siteRepo}/public/models/${m.model}.glb`,
        path: `public/models/${m.model}.glb`,
        modifiedAt: new Date(clock.nowMs() - 3 * 86_400_000).toISOString(),
        bytes: Math.round((m.bytes ?? 0) * k),
        triangles: Math.round(m.triangles * k),
        gpuBytes: Math.round((m.gpuBytes ?? 0) * 1.6),
        materials: m.materials + 1,
      };
    });
  const stops = view.stops.stops.map((stop, i) => ({ ...stop, poster: posterPath(projectDir, i) }));
  return { ...view, models: [...view.models, ...site], stops: { ...view.stops, stops } };
}

export function createPageHandlers(ctx: PageContext) {
  const at = addresses(ctx.state);
  const settings = new Map<string, PageSettings>();
  let server: ServerStatus = IDLE;
  let timer: ReturnType<typeof setTimeout> | null = null;

  if (ctx.state === 'no-site') {
    try {
      ctx.saveProject({ ...ctx.project('moon-island'), siteRepo: null });
    } catch {
      // No workspace in this scenario.
    }
  }

  const change = (next: ServerStatus) => {
    server = next;
    ctx.emit(server);
  };

  const settingsOf = (id: string) => settings.get(id) ?? DEFAULTS;

  function view(id: string): PageView {
    const p = ctx.project(id);
    const s = settingsOf(id);
    const mine = server.project === id;
    return {
      project: id,
      siteRepo: p.siteRepo,
      settings: s,
      command: p.siteRepo ? detected(s) : null,
      commandError: p.siteRepo
        ? null
        : 'This project has no site repo yet. Pick one in Settings > Project.',
      siteConfig: null,
      server: mine ? server : IDLE,
      busyWith: !mine && server.state !== 'idle' ? server.project : null,
    };
  }

  function stopServer() {
    if (timer) clearTimeout(timer);
    timer = null;
    change(IDLE);
  }

  function start(id: string, owner: ServerStatus['owner']) {
    const p = ctx.project(id);
    if (!p.siteRepo)
      fail('invalid', 'This project has no site repo yet. Pick one in Settings > Project.');
    if (server.project === id && (server.state === 'running' || server.state === 'starting')) {
      return;
    }
    if (timer) clearTimeout(timer);
    const command = detected(settingsOf(id))
      .argv.map(a => (a === '{port}' ? '4321' : a))
      .join(' ');
    const log = startLog(command);
    change({
      ...IDLE,
      state: 'starting',
      project: id,
      site: p.siteRepo,
      command,
      owner,
      startedAt: clock.now().toISOString(),
      log: log.slice(0, 4),
    });
    if (ctx.state === 'starting') return;
    timer = setTimeout(() => {
      timer = null;
      if (ctx.state === 'failed') {
        const tail = [
          ...log.slice(0, 4),
          "Error: Cannot find module 'astro/package.json'",
          '  Did you install the site packages? Run pnpm install in the site repo.',
          ' ELIFECYCLE  Command failed with exit code 1.',
        ];
        change({
          ...server,
          state: 'failed',
          message: `The site's dev server stopped before it answered (exit code 1).\n${tail.slice(-3).join('\n')}`,
          log: tail,
        });
        return;
      }
      change({
        ...server,
        state: 'running',
        ...at,
        log: [...log, '', `  Local    ${at.url}`, '  Network  use --host to expose'],
      });
    }, ctx.startMs);
  }

  const handlers: Record<string, (args: Args) => unknown> = {
    page_get: ({ project }) => view(String(project)),
    page_save: ({ project, settings: next }) => {
      const id = String(project);
      ctx.project(id);
      const s = next as PageSettings;
      const concept = s.concept?.trim() || null;
      if (concept && !/\.html?$/i.test(concept))
        fail('invalid', 'A page concept is an .html file.');
      if (s.source === 'concept' && !concept) fail('invalid', 'Pick a page concept first.');
      if (!s.path.trim().startsWith('/')) fail('invalid', 'The path starts with /.');
      settings.set(id, {
        ...DEFAULTS,
        ...s,
        devCommand: s.devCommand?.trim() || null,
        concept,
        path: s.path.trim() || '/',
      });
      return view(id);
    },
    page_server_start: ({ project }) => {
      start(String(project), 'page');
      return view(String(project));
    },
    page_server_stop: ({ project }) => {
      if (server.project === String(project)) stopServer();
      return view(String(project));
    },
    page_server_release: ({ project }) => {
      if (server.project !== String(project) || server.owner !== 'page') return false;
      stopServer();
      return true;
    },
    page_open_in_browser: ({ project, path }) => {
      if (server.project !== String(project) || server.state !== 'running') {
        fail('invalid', "The site's dev server is not running yet.");
      }
      return `${at.url.replace(/\/$/, '')}${String(path)}`;
    },
    dialog_pick_file: ({ title }) =>
      /html/i.test(String(title))
        ? conceptPath(ctx.projectDir(String(ctx.project('moon-island').id)))
        : '/usr/local/bin/toktx',
  };
  /** With `?page=` the site holds the last ship, for the Page view's compare. */
  function wrapInspect(all: Record<string, (args: Args) => unknown>) {
    const inspect = all.inspect_project;
    if (!ctx.state || !inspect) return;
    all.inspect_project = args => {
      const id = String(args.project);
      const site = ctx.project(id).siteRepo;
      const found = inspect(args) as InspectView;
      return site && found.models.length ? withShip(found, site, ctx.projectDir(id)) : found;
    };
  }

  return { handlers, wrapInspect };
}
