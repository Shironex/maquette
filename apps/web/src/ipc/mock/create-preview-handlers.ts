/**
 * Previews in mock mode: Import models and Render previews give each model
 * without one a render, as the thumbnail service would once Blender is done.
 * The pictures are the bundled Board renders, served at the project's
 * `thumbs/<asset>_v<n>.png`.
 */
import type { Asset, PreviewsAsked, ProjectManifest } from '../client';
import { wantsPreview } from '@/state/thumbs';
import { refMayReplace } from './lab';
import { expand, type Args, type BackendOptions } from './backend-options';
import { DEFAULT_ROOT } from './fixtures';
import { thumbsDir } from './inbox-fixtures';

const PROP_RENDERS = [
  'guardian-statue',
  'crystal-lantern',
  'observatory-arch',
  'rune-obelisk',
  'ice-crystals',
  'veil-gate',
  'veil-platform',
];

function renderFor(asset: Asset, i: number): string {
  const name =
    asset.kind === 'character'
      ? 'elsheria'
      : (PROP_RENDERS[i % PROP_RENDERS.length] ?? 'veil-platform');
  return `${thumbsDir(`${expand(DEFAULT_ROOT)}/inbox`)}/${name}.webp`;
}

const SLOW_PREVIEW_MS = 2500;

/** The project with three of its models' pictures gone, as one looks before Render previews. */
export function withoutPreviews(p: ProjectManifest): ProjectManifest {
  let left = 3;
  const assets = (p.assets ?? []).map(asset => {
    if (left === 0 || asset.versions.length === 0) return asset;
    left -= 1;
    return { ...asset, thumbnail: null };
  });
  return { ...p, assets };
}

export function createPreviewHandlers(deps: {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  files: BackendOptions['files'];
  emit?: BackendOptions['emit'];
  /** `slow`: Render previews answers at once and each render lands a while later, as Blender's do. */
  previews?: string | null;
}) {
  /** The project with a preview for each asset in `only` (all when null) that has none. */
  function withPreviews(p: ProjectManifest, only: readonly string[] | null) {
    let asked = 0;
    const assets = (p.assets ?? []).map((asset, i) => {
      const latest = asset.versions[asset.versions.length - 1];
      const wanted = !only || only.includes(asset.id);
      if (!latest || !wanted || !refMayReplace(asset.thumbnail)) return asset;
      asked += 1;
      const thumbnail = `thumbs/${asset.id.replaceAll('-', '_')}_v${latest.version}.png`;
      deps.files?.alias(
        `${expand(DEFAULT_ROOT)}/projects/${p.id}/${thumbnail}`,
        renderFor(asset, i)
      );
      return { ...asset, thumbnail };
    });
    return {
      asked,
      rendering: 0,
      failed: 0,
      reason: null,
      project: asked ? deps.saveProject({ ...p, assets }) : p,
    };
  }

  /** Asks for every missing preview now and lands them one at a time. */
  function renderSlowly(p: ProjectManifest): PreviewsAsked {
    const ids = (p.assets ?? []).filter(wantsPreview).map(a => a.id);
    const root = `${expand(DEFAULT_ROOT)}/projects/${p.id}`;
    ids.forEach((asset, i) => {
      setTimeout(
        () => {
          const { project } = withPreviews(deps.project(p.id), [asset]);
          const found = project.assets?.find(a => a.id === asset);
          const latest = found?.versions[found.versions.length - 1];
          const ready = {
            file: `${root}/${latest?.file ?? ''}`,
            bytes: null,
            modifiedMs: null,
            thumbnail: found?.thumbnail ?? null,
            state: 'ready' as const,
          };
          deps.emit?.({ name: 'thumbnail', payload: { ...ready, project: null } });
          deps.emit?.({ name: 'thumbnail', payload: { ...ready, project } });
        },
        (i + 1) * SLOW_PREVIEW_MS
      );
    });
    return { asked: ids.length, rendering: ids.length, failed: 0, reason: null, project: p };
  }

  return {
    withPreviews,
    handlers: {
      thumbnail_render_missing: ({ project: id }: Args): PreviewsAsked => {
        if (deps.previews !== 'slow') return withPreviews(deps.project(String(id)), null);
        return renderSlowly(deps.project(String(id)));
      },
    },
  };
}
