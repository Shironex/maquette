/**
 * A fake Concept lab for mock mode: jobs "generate" by waiting a few
 * seconds and then serving one of the bundled concept images at their
 * path, a few at a time like the real Codex lane.
 */
import type {
  AssetKind,
  ConceptEvent,
  ConceptJob,
  Direction,
  LabView,
  NewConcepts,
  ProjectManifest,
  RefImage,
  SentPicks,
} from '../client';
import { GENERATED_POOL, labDirections, labRefs, runningJobs } from './lab-fixtures';

type Args = Record<string, unknown>;

export interface FakeLabOptions {
  /** Absolute folder of a project. */
  projectDir: (project: string) => string;
  project: (id: string) => ProjectManifest;
  saveProject: (project: ProjectManifest) => ProjectManifest;
  emit?: (event: ConceptEvent) => void;
  files?: {
    alias: (path: string, from: string) => void;
    /** Tiles `sources` into one PNG served at `path`. */
    sheet?: (path: string, sources: string[]) => Promise<void>;
  };
  /** How long one fake image takes. */
  jobMs: number;
  /** `running`: the porch direction has two images generating that never finish. */
  state?: string | null;
  fail: (code: string, message: string) => never;
}

const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'project';

export function createFakeLab(options: FakeLabOptions) {
  const { fail } = options;
  const running = options.state === 'running';
  const directions = new Map<string, Direction[]>([['moon-island', labDirections(running)]]);
  const refs = new Map<string, RefImage[]>([['moon-island', labRefs()]]);
  let jobs: ConceptJob[] = running ? runningJobs('moon-island', Date.now()) : [];
  let concurrency = 4;
  let pool = 0;
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  const list = (project: string) => directions.get(project) ?? [];
  const emit = (event: ConceptEvent) => options.emit?.(structuredClone(event));

  function view(project: string): LabView {
    options.project(project);
    return {
      project,
      projectDir: options.projectDir(project),
      directions: [...list(project)].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)),
      jobs: jobs.filter(j => j.project === project),
      refs: refs.get(project) ?? [],
      concurrency,
    };
  }

  function findDirection(project: string, id: string): Direction {
    const found = list(project).find(d => d.id === id);
    if (!found) return fail('not-found', `Could not find the direction ${id}.`);
    return found;
  }

  function saveDirection(project: string, next: Direction): Direction {
    const others = list(project).filter(d => d.id !== next.id);
    directions.set(project, [next, ...others]);
    return next;
  }

  function update(job: ConceptJob, patch: Partial<ConceptJob>) {
    const next = { ...job, ...patch };
    jobs = jobs.map(j => (j.id === job.id ? next : j));
    emit({ type: 'job', job: next });
    return next;
  }

  function pump() {
    const busy = jobs.filter(j => j.state === 'running' && timers.has(j.id)).length;
    const held = jobs.filter(j => j.state === 'running' && !timers.has(j.id)).length;
    let free = concurrency - busy - held;
    for (const job of jobs.filter(j => j.state === 'queued')) {
      if (free <= 0) break;
      free -= 1;
      const started = update(job, {
        state: 'running',
        startedAtMs: Date.now(),
        logTail: ['OpenAI Codex', 'session id: 01mock', 'Generating the image'],
      });
      const ms = options.jobMs * (0.75 + ((job.index * 37) % 50) / 100);
      timers.set(
        job.id,
        setTimeout(() => finish(started.id), ms)
      );
    }
  }

  function finish(id: string) {
    timers.delete(id);
    const job = jobs.find(j => j.id === id);
    if (!job || job.state !== 'running') return;
    const source = GENERATED_POOL[pool % GENERATED_POOL.length] ?? GENERATED_POOL[0];
    pool += 1;
    const dir = options.projectDir(job.project);
    options.files?.alias(`${dir}/${job.file}`, `${dir}/concepts/__pool/${source}`);
    const direction = findDirection(job.project, job.direction);
    const at = new Date().toISOString();
    const next = saveDirection(job.project, {
      ...direction,
      updatedAt: at,
      images: [
        ...direction.images.filter(i => i.file !== job.file),
        {
          file: job.file,
          index: job.index,
          width: 1672,
          height: 941,
          starred: false,
          picked: false,
          refFile: null,
          createdAt: at,
        },
      ].sort((a, b) => a.index - b.index),
    });
    update(job, {
      state: 'done',
      finishedAtMs: Date.now(),
      width: 1672,
      height: 941,
      logTail: [...job.logTail, `saved ${job.file}`],
    });
    emit({ type: 'direction', project: job.project, direction: next });
    pump();
  }

  function generate(request: NewConcepts) {
    const name = request.direction.trim();
    if (!name) fail('invalid', 'Name the direction.');
    if (!request.prompt.trim()) fail('invalid', 'Describe what the concepts should show.');
    if (request.count < 1 || request.count > 12)
      fail('invalid', 'Ask for between 1 and 12 images.');
    const id = slug(name);
    const at = new Date().toISOString();
    const existing = list(request.project).find(d => d.id === id);
    const direction = saveDirection(request.project, {
      id,
      name,
      prompt: request.prompt.trim(),
      refs: request.refs,
      aspect: request.aspect,
      images: existing?.images ?? [],
      createdAt: existing?.createdAt ?? at,
      updatedAt: at,
    });
    const taken = [
      ...direction.images.map(i => i.index),
      ...jobs.filter(j => j.direction === id).map(j => j.index),
    ];
    const first = Math.max(0, ...taken) + 1;
    const stamp = Date.now().toString(16);
    for (let n = 0; n < request.count; n += 1) {
      const index = first + n;
      const two = String(index).padStart(2, '0');
      const job: ConceptJob = {
        id: `${id}-${two}-${stamp}`,
        runId: `concepts-run-${stamp}-${n}`,
        project: request.project,
        direction: id,
        directionName: name,
        index,
        file: `concepts/${id}/${id}_${two}.png`,
        state: 'queued',
        queuedAtMs: Date.now(),
        startedAtMs: null,
        finishedAtMs: null,
        message: null,
        width: null,
        height: null,
        logTail: [],
      };
      jobs = [...jobs, job];
      emit({ type: 'job', job });
    }
    pump();
    return view(request.project);
  }

  function cancel(id: string) {
    const job = jobs.find(j => j.id === id);
    if (!job) return fail('not-found', `Could not find the concept job ${id}.`);
    if (job.state !== 'queued' && job.state !== 'running') return;
    const timer = timers.get(id);
    if (timer) clearTimeout(timer);
    timers.delete(id);
    update(job, { state: 'cancelled', finishedAtMs: Date.now(), message: 'Cancelled.' });
    pump();
  }

  function sendPicks(project: string, id: string, kind: AssetKind): SentPicks {
    const direction = findDirection(project, id);
    const picks = direction.images.filter(i => i.picked);
    if (picks.length === 0) fail('invalid', 'Pick at least one concept to send to Refs.');
    const dir = options.projectDir(project);
    const copied = picks.map(i => `refs/${i.file.slice(i.file.lastIndexOf('/') + 1)}`);
    picks.forEach((image, n) =>
      options.files?.alias(`${dir}/${copied[n]}`, `${dir}/${image.file}`)
    );
    const next = saveDirection(project, {
      ...direction,
      updatedAt: new Date().toISOString(),
      images: direction.images.map(i =>
        i.picked ? { ...i, refFile: `refs/${i.file.slice(i.file.lastIndexOf('/') + 1)}` } : i
      ),
    });
    const known = refs.get(project) ?? [];
    refs.set(project, [
      ...known.filter(r => !copied.includes(r.file)),
      ...copied.map(file => ({ file, name: file.slice(5), bytes: 2.2e6 })),
    ]);
    const manifest = options.project(project);
    const at = new Date().toISOString();
    const assets = [...(manifest.assets ?? [])];
    const assetId = slug(direction.name);
    const order = ['concept', 'refs', 'tripo', 'blender', 'web', 'ship'];
    let asset = assets.find(a => a.id === assetId);
    if (asset && order.indexOf(asset.stage) < 1) asset = { ...asset, stage: 'refs', updatedAt: at };
    if (!asset) {
      asset = {
        id: assetId,
        name: direction.name,
        kind,
        stage: 'refs',
        status: 'queued',
        versions: [],
        thumbnail: null,
        createdAt: at,
        updatedAt: at,
      };
    }
    const placed = asset;
    const saved = options.saveProject({
      ...manifest,
      assets: assets.some(a => a.id === assetId)
        ? assets.map(a => (a.id === assetId ? placed : a))
        : [...assets, placed],
    });
    return { direction: next, asset: placed, copied, project: saved };
  }

  const handlers: Record<string, (args: Args) => unknown> = {
    lab_get: ({ project }) => view(String(project)),
    lab_generate: ({ request }) => generate(request as NewConcepts),
    lab_cancel: ({ jobId }) => cancel(String(jobId)),
    lab_cancel_all: ({ project }) => {
      const live = jobs.filter(
        j => j.project === project && (j.state === 'queued' || j.state === 'running')
      );
      for (const job of live) cancel(job.id);
      return live.length;
    },
    lab_clear_finished: ({ project }) => {
      jobs = jobs.filter(
        j => j.project !== project || j.state === 'queued' || j.state === 'running'
      );
      return view(String(project));
    },
    lab_mark: ({ project, direction, file, starred, picked }) => {
      const found = findDirection(String(project), String(direction));
      if (!found.images.some(i => i.file === file))
        fail('not-found', `Could not find the concept ${String(file)}.`);
      return saveDirection(String(project), {
        ...found,
        updatedAt: new Date().toISOString(),
        images: found.images.map(i =>
          i.file === file
            ? {
                ...i,
                starred: typeof starred === 'boolean' ? starred : i.starred,
                picked: typeof picked === 'boolean' ? picked : i.picked,
              }
            : i
        ),
      });
    },
    lab_contact_sheet: async ({ project, direction }) => {
      const found = findDirection(String(project), String(direction));
      if (found.images.length === 0) fail('invalid', 'This direction has no images yet.');
      const dir = options.projectDir(String(project));
      const path = `${dir}/concepts/${found.id}/contact_sheet.png`;
      await options.files?.sheet?.(
        path,
        found.images.map(i => `${dir}/${i.file}`)
      );
      return path;
    },
    lab_send_picks: ({ project, direction, kind }) =>
      sendPicks(String(project), String(direction), kind as AssetKind),
    lab_import_refs: ({ project, paths }) => {
      const dir = options.projectDir(String(project));
      const out = (paths as string[]).map(path => {
        const name = path.slice(path.lastIndexOf('/') + 1);
        if (!/\.(png|jpe?g|webp)$/i.test(name))
          fail('invalid', `${name} is not a PNG, JPEG or WebP image.`);
        options.files?.alias(`${dir}/refs/${name}`, path);
        return `refs/${name}`;
      });
      const known = refs.get(String(project)) ?? [];
      refs.set(String(project), [
        ...known.filter(r => !out.includes(r.file)),
        ...out.map(file => ({ file, name: file.slice(5), bytes: 1.4e6 })),
      ]);
      return out;
    },
  };

  return {
    handlers,
    refs: (project: string) => refs.get(project) ?? [],
    sentFor: (project: string, assetName: string) =>
      list(project)
        .filter(d => d.name.trim().toLowerCase() === assetName.trim().toLowerCase())
        .flatMap(d => d.images.flatMap(i => (i.refFile ? [i.refFile] : []))),
    setConcurrency: (n: number) => {
      concurrency = Math.max(1, n);
      pump();
    },
  };
}
