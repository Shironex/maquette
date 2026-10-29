/**
 * The Downloads watcher for mock mode: on, watching the fake Downloads
 * folder, with the waiting By hand sends as its pending hand-offs.
 * `?arrival=named` drops an export named for the first waiting send a
 * moment after the app opens (it is copied into the inbox),
 * `?arrival=unmatched` drops "model (3).glb", which asks which asset it is,
 * and `?arrival=look` drops a Tripo batch export whose name is only close
 * to the Lantern post send (it waits for a look) and two that fit no send.
 */
import type {
  Arrival,
  DownloadPreview,
  SendSuggestion,
  PendingHandoff,
  ProjectManifest,
  ProviderJob,
  WatchStatus,
} from '../client';
import { clock } from '@/lib/clock';
import { expand, fail, type Args } from './backend-options';
import { DEFAULT_ROOT, HOME } from './fixtures';
import { exportName } from './handoff-card';
import { thumbsDir } from './inbox-fixtures';

const DOWNLOADS = `${HOME}/Downloads`;
const ARRIVAL_MS = 1200;
/** The renders mock mode serves as each download's thumbnail, by file name. */
const PREVIEWS: Record<string, string> = {
  'lantern post 3d model.glb': 'crystal-lantern.webp',
  'frost arch 3d model.glb': 'observatory-arch.webp',
  'rune circle 3d model.glb': 'veil-platform.webp',
};
const OPEN: readonly Arrival['state'][] = ['review', 'unmatched', 'failed'];

/** The render mock mode shows as a download's front picture, by file name. */
const renderOf = (name: string): string | null => {
  const render = PREVIEWS[name];
  return render ? `${thumbsDir(`${expand(DEFAULT_ROOT)}/inbox`)}/${render}` : null;
};

function pendingOf(job: ProviderJob): PendingHandoff {
  return {
    job: job.id,
    project: job.project,
    asset: job.asset,
    assetName: job.assetName,
    service: job.service,
    expectedName: job.card?.exportName || exportName(job),
    handoff: job.handoff,
    image: job.images[0] ?? null,
    credits: job.card?.credits ?? null,
    createdAtMs: job.createdAtMs,
  };
}

export function createDownloadsHandlers({
  jobs,
  handBack,
  project,
  emit,
  arrival = null,
  delayMs = ARRIVAL_MS,
}: {
  jobs: () => ProviderJob[];
  /** The provider mock's hand-back: copies a file into the inbox for a send. */
  handBack: (args: Args) => unknown;
  project: (id: string) => ProjectManifest;
  emit: (arrival: Arrival) => void;
  arrival?: string | null;
  /** How long after opening the download lands. */
  delayMs?: number;
}): Record<string, (args: Args) => unknown> {
  let enabled = true;
  let custom: string | null = null;
  let arrivals: Arrival[] = [];
  const waiting = () => jobs().filter(j => j.provider === 'by-hand' && j.state === 'waiting');

  const status = (): WatchStatus => ({
    enabled,
    folder: custom ?? DOWNLOADS,
    custom: custom !== null,
    watching: enabled,
    error: null,
    pending: waiting().map(pendingOf),
    arrivals,
  });

  /**
   * Every waiting send ranked against a download by picture, as the core
   * does once its front render is in: `best` (an asset name) at `top`, the
   * rest falling away from `rest`.
   */
  const ranked = (best: string | null, top: number, rest: number): SendSuggestion[] =>
    waiting()
      .map((job, i): SendSuggestion => {
        const picture = job.assetName === best ? top : Math.max(0.12, rest - i * 0.07);
        const name = job.assetName === best ? 0.6 : 0;
        return {
          job: job.id,
          project: job.project,
          asset: job.asset,
          assetName: job.assetName,
          score: 0.9 * picture + 0.1 * name,
          picture,
          name,
          image: job.images[0] ?? null,
        };
      })
      .sort((a, b) => (b.score ?? 0) - (a.score ?? 0));

  const remember = (next: Arrival): Arrival => {
    arrivals = [next, ...arrivals.filter(a => a.id !== next.id)];
    emit(next);
    return next;
  };

  const file = (name: string, n: number): Arrival => ({
    id: `a1b2c3d4e5f6000${n}`,
    path: `${DOWNLOADS}/${name}`,
    name,
    inner: null,
    bytes: 4.2e6,
    atMs: clock.nowMs(),
    state: 'unmatched',
    job: null,
    project: null,
    asset: null,
    assetName: null,
    reason: '',
    inboxFile: null,
  });

  const take = (base: Arrival, job: ProviderJob, reason: string): Arrival => {
    handBack({ id: job.id, file: base.path });
    return remember({
      ...base,
      state: 'imported',
      job: job.id,
      project: job.project,
      asset: job.asset,
      assetName: job.assetName,
      reason,
      inboxFile: `${job.id}.glb`,
    });
  };

  /** Tripo's batch export: one name close to a send, two that fit none. */
  const batchExport = () => {
    const lantern = waiting().find(j => j.assetName === 'Lantern post');
    const asking = `${waiting().length} sends are waiting`;
    const pictured = (name: string, suggestions: SendSuggestion[]) => ({
      suggestions,
      picture: 'ready' as const,
      render: renderOf(name),
    });
    remember({
      ...file('rune circle 3d model.glb', 5),
      reason: asking,
      ...pictured('rune circle 3d model.glb', ranked(null, 0, 0.41)),
    });
    remember({
      ...file('frost arch 3d model.glb', 4),
      reason: asking,
      ...pictured('frost arch 3d model.glb', ranked(null, 0, 0.52)),
    });
    if (!lantern) return;
    const name = 'lantern post 3d model.glb';
    remember({
      ...file(name, 3),
      state: 'review',
      job: lantern.id,
      project: lantern.project,
      asset: lantern.asset,
      assetName: lantern.assetName,
      reason: `Looks like ${lantern.assetName} (88%)`,
      ...pictured(name, ranked(lantern.assetName, 0.91, 0.47)),
    });
  };

  if (arrival === 'named' || arrival === 'unmatched' || arrival === 'look') {
    setTimeout(() => {
      const [first] = waiting();
      if (!first) return;
      if (arrival === 'named') {
        take(file(`${exportName(first)}.glb`, 1), first, `Named for ${first.assetName}`);
      } else if (arrival === 'look') {
        batchExport();
      } else {
        remember({ ...file('model (3).glb', 2), reason: `${waiting().length} sends are waiting` });
      }
    }, delayMs);
  }

  /**
   * Sends a download to the Inbox for an asset, as the core does: for its
   * newest waiting By hand send, or aimed at the asset when none waits.
   */
  const attach = (found: Arrival, projectId: string, assetId: string): Arrival => {
    const [job] = waiting()
      .filter(j => j.project === projectId && j.asset === assetId)
      .sort((a, b) => (b.createdAtMs ?? 0) - (a.createdAtMs ?? 0));
    const target =
      (project(projectId).assets ?? []).find(a => a.id === assetId) ??
      fail('not-found', `Could not find the asset ${assetId}.`);
    if (job) handBack({ id: job.id, file: found.path });
    return remember({
      ...found,
      state: 'attached',
      job: job?.id ?? null,
      project: projectId,
      asset: target.id,
      assetName: target.name,
      reason: `You attached it to ${target.name}`,
      inboxFile: job ? `${job.id}.glb` : `download-${target.id}.glb`,
    });
  };

  const find = (id: unknown): Arrival =>
    arrivals.find(a => a.id === id) ?? fail('not-found', `Could not find the download ${id}.`);

  return {
    downloads_watch_status: status,
    downloads_watch_set: ({ enabled: on, folder: dir }) => {
      if (typeof dir === 'string' && /missing/i.test(dir))
        fail('invalid', `${dir} is not a folder.`);
      enabled = Boolean(on);
      custom = typeof dir === 'string' && dir.trim() ? dir.trim() : null;
      return status();
    },
    downloads_attach: ({ arrival: id, project: projectId, asset }) => {
      const found = find(id);
      if (!OPEN.includes(found.state)) fail('invalid', `${found.name} was already taken care of.`);
      return attach(found, String(projectId), String(asset));
    },
    // A file someone fixed by hand goes the same way: copied, never moved.
    inbox_add_model_file: ({ project: projectId, asset, path }) => {
      if (!/\.(glb|gltf|fbx|obj)$/i.test(String(path)))
        fail('invalid', `${path} is not a model file. Add a .glb, .gltf, .fbx or .obj.`);
      if (/\/models\//.test(String(path)))
        fail('invalid', 'That file is a version already. Add one from outside the models folder.');
      const target =
        (project(String(projectId)).assets ?? []).find(a => a.id === asset) ??
        fail('not-found', `Could not find the asset ${asset}.`);
      const [job] = waiting()
        .filter(j => j.project === projectId && j.asset === target.id)
        .sort((a, b) => (b.createdAtMs ?? 0) - (a.createdAtMs ?? 0));
      if (job) handBack({ id: job.id, file: String(path) });
      return {
        inboxFile: job ? `${job.id}.glb` : `download-${target.id}.glb`,
        job: job?.id ?? null,
        asset: target.id,
        assetName: target.name,
      };
    },
    downloads_preview: ({ arrival: id }): DownloadPreview => {
      const found = find(id);
      const render = PREVIEWS[found.name];
      return {
        file: `${HOME}/Library/Application Support/Maquette/download-previews/${found.id}.glb`,
        thumbnail: render ? `${thumbsDir(`${expand(DEFAULT_ROOT)}/inbox`)}/${render}` : null,
        state: render ? 'ready' : 'fallback',
      };
    },
    downloads_dismiss: ({ arrival: id }) => {
      const found = find(id);
      const reason =
        found.state === 'review' && found.assetName
          ? `You said it is not ${found.assetName}`
          : 'Not a model for Maquette';
      const next: Arrival = { ...found, state: 'dismissed', reason };
      arrivals = arrivals.map(a => (a.id === next.id ? next : a));
      return next;
    },
  };
}
