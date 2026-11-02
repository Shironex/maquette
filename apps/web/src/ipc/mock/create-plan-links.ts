/**
 * `plan_links` in mock mode: the live read of the assets and Lab
 * directions a canvas links, from the mock project, Inbox and Lab, as the
 * core reads them (crates/maquette-core/src/plan_links.rs). Mock mode has
 * no files to look at, so the Observatory arch stands for an asset whose
 * newest model file is missing.
 */
import type {
  AssetLink,
  DirectionLink,
  InboxItem,
  LabView,
  PlanLinks,
  ProjectManifest,
} from '../client';
import type { Args } from './backend-options';
import type { createFakeLayout } from './layout';

/** Assets whose newest version's file mock mode treats as gone from disk. */
const MISSING_FILES = new Set(['observatory-arch']);

export interface PlanLinkDeps {
  root: () => string | null;
  /**
   * What the links are read from: the projects, the Inbox and the Lab's
   * handlers; and the layout.json the camera plan is built from and applied to.
   */
  live: {
    projects: () => readonly ProjectManifest[];
    inbox: () => readonly InboxItem[];
    lab: { lab_get?: (args: Args) => unknown };
    layout?: ReturnType<typeof createFakeLayout>;
  };
}

function assetLink(deps: PlanLinkDeps, project: string, dir: string, id: string): AssetLink {
  const asset = deps.live
    .projects()
    .find(p => p.id === project)
    ?.assets?.find(a => a.id === id);
  if (!asset) {
    return {
      id,
      state: 'gone',
      name: null,
      kind: null,
      stage: null,
      thumbnail: null,
      version: null,
      file: null,
      stats: null,
      checks: 0,
    };
  }
  const latest = asset.versions.at(-1) ?? null;
  const checks = deps.live
    .inbox()
    .filter(
      i =>
        !i.replacedBy &&
        ((i.link?.project === project && i.link.asset === id) ||
          (i.aim?.project === project && i.aim.asset === id))
    ).length;
  return {
    id,
    state:
      checks > 0
        ? 'check'
        : !latest
          ? 'no-model'
          : MISSING_FILES.has(id)
            ? 'file-missing'
            : 'ready',
    name: asset.name,
    kind: asset.kind,
    stage: asset.stage,
    thumbnail: asset.thumbnail ? `${dir}/${asset.thumbnail}` : null,
    version: latest?.version ?? null,
    file: latest?.file ?? null,
    stats: latest?.stats ?? null,
    checks,
  };
}

function directionLink(view: LabView | null, dir: string, id: string): DirectionLink {
  const found = view?.directions.find(d => d.id === id);
  if (!found) {
    return {
      id,
      state: 'gone',
      name: null,
      images: 0,
      picked: 0,
      pending: 0,
      done: 0,
      thumbnail: null,
    };
  }
  const jobs = view?.jobs.filter(j => j.direction === id) ?? [];
  const finished = ['done', 'failed', 'cancelled'];
  const pending = jobs.filter(j => !finished.includes(j.state)).length;
  // Only a job being made counts as running: held, queued or waiting ones do not.
  const making = jobs.some(j => j.state === 'running');
  const picked = found.images.filter(i => i.picked).length;
  const cover =
    found.images.find(i => i.picked) ??
    [...found.images].sort((a, b) => b.index - a.index)[0] ??
    null;
  return {
    id,
    state: making ? 'running' : picked > 0 ? 'picked' : found.images.length ? 'pick' : 'empty',
    name: found.name,
    images: found.images.length,
    picked,
    pending,
    done: making ? jobs.filter(j => j.state === 'done').length : 0,
    thumbnail: cover ? `${dir}/${cover.file}` : null,
  };
}

export function createPlanLinks(deps: PlanLinkDeps) {
  return {
    plan_links: ({ project, assets, directions }: Args): PlanLinks => {
      const id = String(project);
      const dir = `${deps.root() ?? ''}/projects/${id}`;
      const unique = (ids: unknown) => [...new Set((ids as string[] | undefined) ?? [])];
      const view = (deps.live.lab.lab_get?.({ project: id }) as LabView | undefined) ?? null;
      return {
        assets: unique(assets).map(a => assetLink(deps, id, dir, a)),
        directions: unique(directions).map(d => directionLink(view, dir, d)),
      };
    },
  };
}
