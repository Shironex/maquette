/**
 * The 3D quick view: one model file in a modal over the Board or the
 * Library, with its name, version and numbers, and the way into the full
 * Inspector when the model is a version of an asset in the open project.
 */
import type { Asset, LibraryItem, ModelStats, WorkspaceSnapshot } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';
import { projectFile } from './board';
import { latestOf } from './library';

/** A version of an asset in the open project, as the Inspector opens it. */
export interface InspectTarget {
  project: string;
  asset: string;
  version: number;
}

/** What the quick view shows. */
export interface QuickViewTarget {
  name: string;
  version: number;
  /** The model file, absolute, for `fileUrl`. */
  file: string;
  stats: ModelStats | null;
  /** Where it lives: the project's name, or the library. */
  from: string;
  /** Opens it in the Inspector; null when the Inspector cannot show it. */
  inspect: InspectTarget | null;
  /** Why Open in Inspector is off, when `inspect` is null. */
  inspectBlocked: string | null;
}

/** The viewer reads GLB and glTF files only. */
export const isViewable = (file: string): boolean => /\.(glb|gltf)$/i.test(file);

/** The newest version of a Board card, or null before it has a model. */
export function assetQuickView(
  asset: Asset,
  workspace: WorkspaceSnapshot | null
): QuickViewTarget | null {
  const latest = asset.versions.at(-1);
  const project = workspace?.project;
  if (!latest || !workspace || !project || !isViewable(latest.file)) return null;
  const file = projectFile(workspace, latest.file);
  if (!file) return null;
  return {
    name: asset.name,
    version: latest.version,
    file,
    stats: latest.stats,
    from: project.name,
    inspect: { project: project.id, asset: asset.id, version: latest.version },
    inspectBlocked: null,
  };
}

/**
 * The newest version of a library model. The Inspector measures project
 * models, so it opens the open project's copy, when the project has one.
 */
export function libraryQuickView(
  item: LibraryItem,
  workspace: WorkspaceSnapshot | null
): QuickViewTarget | null {
  const latest = latestOf(item);
  if (!latest || !isViewable(latest.file)) return null;
  const project = workspace?.project ?? null;
  const use = project ? item.uses.find(u => u.project === project.id) : undefined;
  const copy = use ? project?.assets?.find(a => a.id === use.asset) : undefined;
  const copyVersion = copy?.versions.at(-1)?.version;
  return {
    name: item.entry.name,
    version: latest.version,
    file: `${item.dir}/${latest.file}`,
    stats: latest.stats,
    from: 'Library',
    inspect:
      project && copy && copyVersion !== undefined
        ? { project: project.id, asset: copy.id, version: copyVersion }
        : null,
    inspectBlocked: project
      ? `Add it to ${project.name} to inspect it in full`
      : 'Open a project to inspect it in full',
  };
}

/** "v3 · 34.7k triangles · 1.1 MB · Frostveil". */
export function quickViewLine(target: QuickViewTarget): string {
  const s = target.stats;
  return [
    `v${target.version}`,
    s ? `${formatK(s.triangles)} triangles` : null,
    s?.bytes ? formatBytes(s.bytes) : null,
    target.from,
  ]
    .filter(Boolean)
    .join(' · ');
}
