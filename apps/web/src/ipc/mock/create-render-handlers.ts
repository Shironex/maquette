/**
 * Render styles in mock mode, as the core answers them: a plan of a source,
 * previews from the bundled pictures, renders that write one file at a time
 * on a timer and tell the web view after each, stop and go on. Nothing is
 * written; the records live in memory.
 *
 * `?render=` sets what opens: `running` a render held at 7 of 13 files,
 * `failures` a render that hit every problem, `big` a GIF over the target,
 * `no-blender` and `no-ffmpeg` the tool missing from the toolchain.
 */
import type {
  LibraryView,
  LoopFormat,
  PackSource,
  ProjectManifest,
  RenderFile,
  RenderItem,
  RenderRecord,
  RenderRequest,
  RenderSettings,
  RenderShot,
  RenderTarget,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, sleep, type Args } from './backend-options';
import {
  NO_TEXTURE,
  failedRecord,
  fileFor,
  runningRecord,
  shotOf,
  targetsOfLibrary,
  targetsOfProject,
} from './render-fixtures';

interface RenderContext {
  library: () => LibraryView;
  project: (id: string) => ProjectManifest;
  root: () => string;
  emit: (record: RenderRecord) => void;
  /** Milliseconds per file. */
  pace: number;
  state: string | null;
}

const clone = (r: RenderRecord): RenderRecord => structuredClone(r);

export function createRenderHandlers(ctx: RenderContext) {
  const records = new Map<string, RenderRecord>();
  const stopping = new Set<string>();
  let serial = 0;
  let seeded = false;

  const projectDir = (id: string) => `${ctx.root()}/projects/${id}`;
  const outDir = (source: PackSource) =>
    source.from === 'project'
      ? `${projectDir(source.project)}/exports/renders`
      : `${ctx.root()}/exports/renders`;
  const targets = (source: PackSource): RenderTarget[] => {
    if (source.from === 'project') {
      return targetsOfProject(ctx.project(source.project), projectDir(source.project));
    }
    return targetsOfLibrary(ctx.library(), source.from === 'entries' ? source.entries : []);
  };

  const seed = () => {
    if (seeded) return;
    seeded = true;
    if (ctx.state !== 'running' && ctx.state !== 'failures') return;
    const source: PackSource = { from: 'project', project: 'frostveil' };
    const found = targets(source);
    const make = ctx.state === 'running' ? runningRecord : failedRecord;
    const record = make('frostveil', outDir(source), found);
    records.set(record.id, record);
  };
  const record = (id: string) => {
    seed();
    return records.get(id) ?? fail('not-found', `Could not find the render ${id}.`);
  };
  const tell = (r: RenderRecord) => {
    records.set(r.id, r);
    ctx.emit(clone(r));
    return clone(r);
  };

  /** Writes the next queued file, then the next, until none or a stop. */
  const step = async (id: string) => {
    await sleep(ctx.pace);
    const r = record(id);
    if (r.state !== 'running') return;
    if (stopping.delete(id)) {
      const files = [...r.items.flatMap(i => i.files), ...(r.sheet ? [r.sheet] : [])];
      for (const f of files) if (f.state === 'running') f.state = 'queued';
      tell({ ...r, state: 'stopped' });
      return;
    }
    const files = [...r.items.flatMap(i => i.files), ...(r.sheet ? [r.sheet] : [])];
    const running = files.find(f => f.state === 'running');
    if (running) {
      running.state = 'done';
      running.bytes = running.style === 'turntable' ? 1_320_000 : 840_000;
      running.seconds = 6.2;
    }
    const next = files.find(f => f.state === 'queued');
    if (next) {
      next.state = 'running';
      tell(r);
      void step(id);
      return;
    }
    tell({ ...r, state: 'done', finishedAt: new Date(clock.nowMs()).toISOString() });
  };

  const itemOf = (t: RenderTarget, request: RenderRequest, dir: string): RenderItem | null => {
    const row = request.rows.find(x => x.key === t.key);
    if (!row) return null;
    const ext = request.settings.format ?? 'webp';
    const files: RenderFile[] = row.styles.map(style =>
      style === 'toon' && !t.hasColour
        ? fileFor(dir, t.folder, style, ext, 'skipped', NO_TEXTURE(t.name))
        : fileFor(dir, t.folder, style, ext)
    );
    return { target: t, files };
  };

  const handlers: Record<string, (args: Args) => unknown> = {
    render_plan: ({ source }) => {
      const s = source as PackSource;
      const found = targets(s);
      return {
        label: s.from === 'project' ? `of ${ctx.project(s.project).name}` : 'from the library',
        dir: outDir(s),
        targets: found,
        skipped: [],
      };
    },
    render_preview: async ({ source, key, settings, shots }) => {
      const s = source as PackSource;
      const t = targets(s).find(x => x.key === key);
      if (!t) fail('not-found', `Could not find the model ${String(key)}.`);
      if (ctx.state === 'no-blender') {
        fail(
          'invalid',
          'Render styles use headless Blender. Set its path in Settings, Toolchain, then render again.'
        );
      }
      await sleep(ctx.pace / 2);
      const set = settings as RenderSettings;
      const format: LoopFormat = set.format ?? 'webp';
      return {
        key: t!.key,
        name: t!.name,
        version: t!.version,
        heightM: t!.heightM,
        hasColour: t!.hasColour,
        folder: t!.folder,
        dir: outDir(s),
        shots: (shots as RenderShot[]).map(shot =>
          shotOf(
            ctx.root(),
            shot,
            format,
            set.width ?? 480,
            ctx.state === 'big' && !set.tightCrop,
            Boolean(set.tightCrop)
          )
        ),
      };
    },
    render_start: ({ request }) => {
      const r = request as RenderRequest;
      serial += 1;
      const dir = outDir(r.source);
      const items = targets(r.source).flatMap(t => {
        const item = itemOf(t, r, dir);
        return item ? [item] : [];
      });
      const sheet = r.sheet ? fileFor(dir, '.', 'contact-sheet') : null;
      const made: RenderRecord = {
        version: 1,
        id: `render-${serial}`,
        source: r.source,
        label: 'of the project',
        dir,
        settings: r.settings,
        state: 'running',
        items,
        sheet: sheet ? { ...sheet, file: `${dir}/contact.png` } : null,
        createdAt: new Date(clock.nowMs()).toISOString(),
        finishedAt: null,
        message: null,
      };
      const first = [...items.flatMap(i => i.files), ...(made.sheet ? [made.sheet] : [])].find(
        f => f.state === 'queued'
      );
      if (first) first.state = 'running';
      records.set(made.id, made);
      void step(made.id);
      return clone(made);
    },
    render_get: ({ id }) => clone(record(String(id))),
    render_recent: () => {
      seed();
      return [...records.values()].map(clone);
    },
    render_stop: ({ id }) => {
      const r = record(String(id));
      if (r.id === 'render-running') {
        for (const f of r.items.flatMap(i => i.files)) {
          if (f.state === 'running') f.state = 'queued';
        }
        return tell({ ...r, state: 'stopped' });
      }
      stopping.add(r.id);
      return clone(r);
    },
    render_resume: ({ id }) => {
      const r = record(String(id));
      const go: RenderRecord = { ...r, state: 'running' };
      const next = go.items.flatMap(i => i.files).find(f => f.state === 'queued');
      if (next) next.state = 'running';
      const told = tell(go);
      void step(go.id);
      return told;
    },
    render_reveal: () => null,
  };

  return { handlers };
}
