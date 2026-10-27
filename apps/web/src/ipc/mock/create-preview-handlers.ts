/**
 * Previews in mock mode: Import models and Render previews give each model
 * without one a render, as the thumbnail service would once Blender is done.
 * The pictures are the bundled Board renders, served at the project's
 * `thumbs/<asset>_v<n>.png`.
 */
import type { Asset, PreviewsAsked, ProjectManifest } from '../client';
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

export function createPreviewHandlers(deps: {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  files: BackendOptions['files'];
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

  return {
    withPreviews,
    handlers: {
      thumbnail_render_missing: ({ project: id }: Args): PreviewsAsked =>
        withPreviews(deps.project(String(id)), null),
    },
  };
}
