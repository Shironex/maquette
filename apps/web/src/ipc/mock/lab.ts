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
  NewObjectRefs,
  ProjectManifest,
  RefImage,
  StyleBoardChange,
} from '../client';
import {
  GENERATED_POOL,
  type LabLimit,
  VEIL_GATE_OBJECTS,
  labDirections,
  labRefs,
  objectRefs,
  runningJobs,
  veilGateDirections,
} from './lab-fixtures';
import { clock } from '@/lib/clock';
import { createLabSends } from './create-lab-sends';
import { createLabRequests } from './lab-requests';
import { createRefsHandlers } from './create-refs-handlers';
import { createStyleBoards, markStyled } from './style-board';
import { finished, importRefs, queuedJob, refMayReplace, retryable } from './lab-helpers';

export { refMayReplace };

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
  /**
   * `running`: the arena direction has two images generating that never
   * finish; `limited` adds a rate limited job, `usage` three waiting for the
   * Codex usage limit, `approval` two waiting for an agent's OpenAI Images
   * round to be approved and one OpenAI refused; `refs` tags the guardian statues' refs (see
   * create-refs-handlers.ts).
   */
  state?: string | null;
  /** `batch`: the veil gate props concept and its six object refs are there too. */
  handoff?: string | null;
  /** `board`: Frostveil has a style board (see style-board.ts). */
  style?: string | null;
  /** True while a Send to 3D job still needs the asset where it is. */
  assetBusy?: (project: string, asset: string) => boolean;
  fail: (code: string, message: string) => never;
}

export function createFakeLab(options: FakeLabOptions) {
  const { fail } = options;
  const limit: LabLimit | undefined =
    options.state === 'limited'
      ? 'rate'
      : options.state === 'usage'
        ? 'usage'
        : options.state === 'approval'
          ? 'approval'
          : undefined;
  const running = options.state === 'running' || limit !== undefined;
  const batch = options.handoff === 'batch';
  const seeded = [...(batch ? veilGateDirections() : []), ...labDirections(running)];
  const directions = new Map<string, Direction[]>([
    ['frostveil', options.style === 'board' ? markStyled(seeded) : seeded],
  ]);
  const styles = createStyleBoards(options);
  const refs = new Map<string, RefImage[]>([
    ['frostveil', [...labRefs(), ...(batch ? objectRefs() : [])]],
  ]);
  let jobs: ConceptJob[] = running ? runningJobs('frostveil', clock.nowMs(), limit) : [];
  let concurrency = 4;
  let pool = 0;
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const standIns = new Map<string, string>();

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
      style: styles.board(project),
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
        startedAtMs: clock.nowMs(),
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
    // A refs job shows its stand-in, a veil gate object its crop, anything else the next pool image.
    const crop =
      standIns.get(job.direction) ??
      VEIL_GATE_OBJECTS.find(o => o.title.toLowerCase() === job.title?.toLowerCase())?.image;
    const source = crop ?? GENERATED_POOL[pool % GENERATED_POOL.length] ?? GENERATED_POOL[0];
    if (!crop) pool += 1;
    const square = findDirection(job.project, job.direction).aspect === '1:1';
    const [width, height] = square ? [1024, 1024] : [1672, 941];
    const dir = options.projectDir(job.project);
    options.files?.alias(`${dir}/${job.file}`, `${dir}/concepts/__pool/${source}`);
    const direction = findDirection(job.project, job.direction);
    const at = clock.now().toISOString();
    const next = saveDirection(job.project, {
      ...direction,
      updatedAt: at,
      images: [
        ...direction.images.filter(i => i.file !== job.file),
        {
          file: job.file,
          index: job.index,
          width,
          height,
          starred: false,
          picked: false,
          refFile: null,
          createdAt: at,
          title: job.title,
        },
      ].sort((a, b) => a.index - b.index),
    });
    update(job, {
      state: 'done',
      finishedAtMs: clock.nowMs(),
      width,
      height,
      logTail: [...job.logTail, `saved ${job.file}`],
    });
    emit({ type: 'direction', project: job.project, direction: next });
    pump();
  }

  /** Queues one job per title in a saved direction, numbered after its images and jobs. */
  function enqueue(project: string, direction: Direction, titles: (string | null)[]) {
    const { id, name, styleRevision } = direction;
    const taken = [
      ...direction.images.map(i => i.index),
      ...jobs.filter(j => j.direction === id).map(j => j.index),
    ];
    const first = Math.max(0, ...taken) + 1;
    const stamp = clock.nowMs().toString(16);
    titles.forEach((title, n) => {
      const index = first + n;
      const two = String(index).padStart(2, '0');
      const job = queuedJob({
        id: `${id}-${two}-${stamp}`,
        runId: `concepts-run-${stamp}-${n}`,
        project,
        direction: id,
        directionName: name,
        index,
        file: `concepts/${id}/${id}_${two}.png`,
        title,
        styleRevision,
      });
      jobs = [...jobs, job];
      emit({ type: 'job', job });
    });
    pump();
    return view(project);
  }

  const { generate, makeObjects } = createLabRequests({
    fail,
    list,
    saveDirection,
    stamp: styles.stamp,
    enqueue,
  });

  function cancel(id: string) {
    const job = jobs.find(j => j.id === id);
    if (!job) return fail('not-found', `Could not find the concept job ${id}.`);
    if (finished(job)) return;
    const timer = timers.get(id);
    if (timer) clearTimeout(timer);
    timers.delete(id);
    update(job, { state: 'cancelled', finishedAtMs: clock.nowMs(), message: 'Cancelled.' });
    pump();
  }

  /** Like the core: the same job, prompt and file run again, no new direction. */
  function retry(id: string) {
    const job = jobs.find(j => j.id === id);
    if (!job) return fail('not-found', `Could not find the concept job ${id}.`);
    if (!retryable(job))
      fail('invalid', `${job.file} is ${job.state}; only a failed job runs again.`);
    const again = update(job, {
      state: 'queued',
      queuedAtMs: clock.nowMs(),
      retries: 0,
      retryAtMs: null,
      usageLimit: false,
      startedAtMs: null,
      finishedAtMs: null,
      message: null,
      logTail: [],
    });
    pump();
    return jobs.find(j => j.id === id) ?? again;
  }

  const sends = createLabSends({
    options,
    directions,
    refs,
    jobs: () => jobs,
    list,
    findDirection,
    saveDirection,
  });

  const handlers: Record<string, (args: Args) => unknown> = {
    lab_move_direction: ({ project, direction, to }) =>
      sends.moveDirection(String(project), String(direction), String(to)),
    lab_get: ({ project }) => view(String(project)),
    lab_generate: ({ request }) => generate(request as NewConcepts),
    lab_cancel: ({ jobId }) => cancel(String(jobId)),
    lab_retry: ({ jobId }) => retry(String(jobId)),
    lab_cancel_all: ({ project }) => {
      const live = jobs.filter(j => j.project === project && !finished(j));
      for (const job of live) cancel(job.id);
      return live.length;
    },
    lab_clear_finished: ({ project }) => {
      jobs = jobs.filter(j => j.project !== project || !finished(j));
      return view(String(project));
    },
    lab_mark: ({ project, direction, file, starred, picked }) => {
      const found = findDirection(String(project), String(direction));
      if (!found.images.some(i => i.file === file))
        fail('not-found', `Could not find the concept ${String(file)}.`);
      return saveDirection(String(project), {
        ...found,
        updatedAt: clock.now().toISOString(),
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
      sends.sendPicks(String(project), String(direction), kind as AssetKind),
    lab_make_objects: ({ request }) => makeObjects(request as NewObjectRefs),
    lab_send_object_picks: ({ project, direction, kind }) =>
      sends.sendObjectPicks(String(project), String(direction), (kind as AssetKind | null) ?? null),
    lab_set_style: ({ project, change }) => styles.set(String(project), change as StyleBoardChange),
    lab_import_refs: ({ project, paths }) =>
      importRefs(options, refs, String(project), paths as string[]),
    ...createRefsHandlers({
      options,
      refs,
      list,
      saveDirection,
      enqueue,
      standIns,
      jobs: () => jobs,
    }),
  };

  return {
    handlers,
    /** Concept images queued or generating in `project`. */
    activeJobs: (project: string) => jobs.filter(j => j.project === project && !finished(j)).length,
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
