import type {
  ProjectActivity,
  ProjectManifest,
  ProjectRemoved,
  RemovedProject,
  WorkspaceConfig,
  WorkspaceSnapshot,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';

interface ProjectTrashContext {
  snapshot: () => WorkspaceSnapshot;
  project: (id: string) => ProjectManifest;
  activity: (p: ProjectManifest) => ProjectActivity;
  projects: { get: () => ProjectManifest[]; set: (next: ProjectManifest[]) => void };
  config: { get: () => WorkspaceConfig; set: (next: WorkspaceConfig) => void };
}

const count = (n: number, one: string, many: string) => (n === 1 ? `1 ${one}` : `${n} ${many}`);

/** Why a project cannot be removed now, as the core words it, or null. */
export function removeRefusal(name: string, activity: ProjectActivity): string | null {
  if (activity.run)
    return `A pipeline run is going in ${name}. Wait for it or cancel it in Pipeline, then remove the project.`;
  if (activity.concepts > 0)
    return `${count(activity.concepts, 'concept image is', 'concept images are')} queued or generating in ${name}. Cancel them in the Lab or let them finish, then remove the project.`;
  if (activity.sends > 0)
    return `${count(activity.sends, 'send is', 'sends are')} not over in ${name}: waiting for approval, queued, being made or waiting for a By hand model. Cancel or stop them in Sends, or wait for the Inbox, then remove the project. Their records stay either way.`;
  return null;
}

/**
 * Remove project and Removed projects over an in-memory trash: a removed
 * project leaves the list and waits until it is restored. Like the core it
 * refuses while work runs in the project and opens the next project when
 * the open one goes.
 */
export function createProjectTrashHandlers(
  ctx: ProjectTrashContext
): Record<string, (args: Args) => unknown> {
  let removed: { entry: RemovedProject; manifest: ProjectManifest }[] = [];
  return {
    project_remove: ({ id }): ProjectRemoved => {
      const p = ctx.project(String(id));
      const why = removeRefusal(p.name, ctx.activity(p));
      if (why) fail('invalid', why);
      const list = ctx.projects.get();
      const at = list.findIndex(x => x.id === p.id);
      const stamp = clock.now().toISOString();
      const digits = stamp.replace(/\D/g, '');
      const trash = `${p.id}-${digits.slice(0, 8)}-${digits.slice(8)}`;
      const entry: RemovedProject = {
        trash,
        id: p.id,
        name: p.name,
        target: p.target ?? 'website',
        removedAt: stamp,
        assets: (p.assets ?? []).length,
        path: `trash/_projects/${trash}`,
      };
      removed = [{ entry, manifest: p }, ...removed];
      ctx.projects.set(list.filter(x => x.id !== p.id));
      const config = ctx.config.get();
      if (config.currentProject === p.id) {
        const next = list[at + 1] ?? list[at - 1] ?? null;
        ctx.config.set({ ...config, currentProject: next?.id ?? null });
      }
      return { removed: entry, workspace: ctx.snapshot() };
    },
    project_removed_list: () => {
      ctx.snapshot();
      return removed.map(r => r.entry);
    },
    project_restore: ({ trash }) => {
      const found = removed.find(r => r.entry.trash === trash);
      if (!found) fail('not-found', `Could not find the removed project ${String(trash)}.`);
      if (ctx.projects.get().some(p => p.id === found.entry.id))
        fail(
          'invalid',
          `A project with the id ${found.entry.id} is in the workspace again, so ${found.entry.name} stays in the trash. Remove that project first to restore this one.`
        );
      removed = removed.filter(r => r !== found);
      // Oldest first, as the core lists them.
      const back = [...ctx.projects.get(), found.manifest];
      ctx.projects.set(back.sort((a, b) => a.createdAt.localeCompare(b.createdAt)));
      ctx.config.set({ ...ctx.config.get(), currentProject: found.entry.id });
      return ctx.snapshot();
    },
  };
}
