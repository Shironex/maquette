import type {
  Asset,
  AssetsRemoved,
  ProjectManifest,
  ProviderJob,
  RemoveOutcome,
  TrashEntry,
} from '../client';
import { clock } from '@/lib/clock';
import { PROVIDER_LABEL } from '@/state/providers';
import { MAX_REMOVE } from '@/state/trash';
import { fail, type Args } from './backend-options';

interface TrashContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  jobs: () => readonly ProviderJob[];
  /** Stops a send the way the 3D queue's Cancel and Stop waiting do. */
  stopJob: (id: string) => void;
  /** The run going in the project, if any. */
  activeRun: (project: string) => string | null | undefined;
}

interface Removed {
  entry: TrashEntry;
  asset: Asset;
}

const WAITING_STATES: readonly ProviderJob['state'][] = ['approval', 'queued', 'waiting'];

function outcome(asset: string, name: string | null, over: Partial<RemoveOutcome>): RemoveOutcome {
  return {
    asset,
    name,
    state: 'removed',
    reason: null,
    trash: null,
    trashPath: null,
    files: 0,
    stopped: [],
    warnings: [],
    ...over,
  };
}

/**
 * Remove from project and Restore over an in-memory trash: removed assets
 * leave the project and wait in a list until they are restored.
 */
export function createTrashHandlers(ctx: TrashContext): Record<string, (args: Args) => unknown> {
  const trash = new Map<string, Removed[]>();
  const refuseWhileRunning = (p: ProjectManifest) => {
    const run = ctx.activeRun(p.id);
    if (run)
      fail(
        'invalid',
        `A pipeline run (${run}) is going in ${p.name}. Wait for it to finish or cancel it, then try again.`
      );
  };
  const removeOne = (projectId: string, id: string): RemoveOutcome => {
    const p = ctx.project(projectId);
    const asset = (p.assets ?? []).find(a => a.id === id);
    if (!asset) return outcome(id, null, { state: 'not-found' });
    const mine = ctx.jobs().filter(j => j.project === projectId && j.asset === id);
    const running = mine.find(j => j.state === 'running');
    if (running) {
      return outcome(id, asset.name, {
        state: 'refused',
        reason: `${PROVIDER_LABEL[running.provider]} is making the model of ${asset.name} now (send ${running.id}). Wait for it to reach the Inbox, or cancel it in the 3D queue, then remove the asset.`,
      });
    }
    const stopped = mine.filter(j => WAITING_STATES.includes(j.state)).map(j => j.id);
    stopped.forEach(ctx.stopJob);
    const stamp = clock.now().toISOString();
    const digits = stamp.replace(/\D/g, '');
    const trashId = `${id}-${digits.slice(0, 8)}-${digits.slice(8)}`;
    const files = asset.versions.length * 2 + (asset.thumbnail ? 1 : 0) + stopped.length;
    const path = `trash/${projectId}/${trashId}`;
    const entry: TrashEntry = {
      id: trashId,
      project: projectId,
      asset: id,
      name: asset.name,
      kind: asset.kind,
      versions: asset.versions.length,
      removedAt: stamp,
      files,
      path,
    };
    trash.set(projectId, [{ entry, asset }, ...(trash.get(projectId) ?? [])]);
    ctx.saveProject({
      ...ctx.project(projectId),
      assets: (p.assets ?? []).filter(a => a.id !== id),
    });
    return outcome(id, asset.name, { trash: trashId, trashPath: path, files, stopped });
  };

  return {
    asset_remove: ({ project: id, ids }) => {
      const list = (ids as string[] | undefined) ?? [];
      if (list.length === 0) fail('invalid', 'Name at least one asset to remove.');
      if (list.length > MAX_REMOVE)
        fail('invalid', `Remove at most ${MAX_REMOVE} assets at a time.`);
      refuseWhileRunning(ctx.project(String(id)));
      const outcomes = [...new Set(list)].map(asset => removeOne(String(id), asset));
      const done: AssetsRemoved = { project: ctx.project(String(id)), outcomes };
      return done;
    },
    asset_trash_list: ({ project: id }) => {
      ctx.project(String(id));
      return (trash.get(String(id)) ?? []).map(r => r.entry);
    },
    asset_restore: ({ project: id, trash: trashId }) => {
      const p = ctx.project(String(id));
      refuseWhileRunning(p);
      const list = trash.get(p.id) ?? [];
      const found = list.find(r => r.entry.id === trashId);
      if (!found) fail('not-found', `Could not find the removed asset ${String(trashId)}.`);
      if ((p.assets ?? []).some(a => a.id === found.asset.id || a.name === found.asset.name))
        fail(
          'invalid',
          `An asset called ${found.asset.name} is on the Board again. Rename or remove it, then restore.`
        );
      trash.set(
        p.id,
        list.filter(r => r !== found)
      );
      const project = ctx.saveProject({ ...p, assets: [...(p.assets ?? []), found.asset] });
      return { project, asset: found.asset, files: found.entry.files, stopped: [], warnings: [] };
    },
  };
}
