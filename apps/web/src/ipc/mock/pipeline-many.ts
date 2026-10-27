/**
 * `?pipeline=many`: Frostveil's runner after a build only run of 24
 * models that all came from the Inbox, so every one skips the bake. The
 * Pipeline view's Models table and the folded Baked light card are checked
 * against it.
 */
import type { ModelFile, PipelineView, ProjectManifest } from '../client';
import { manyAssets } from './many-assets-fixtures';

/** The four Frostveil models plus these make 24. */
const EXTRA = 20;

const extra = () => manyAssets().slice(0, EXTRA);

/** The project with its extra assets, when the option asks for them. */
export function seedManyPipeline(project: ProjectManifest, state: string | null): ProjectManifest {
  if (state !== 'many' || project.id !== 'frostveil') return project;
  return { ...project, assets: [...(project.assets ?? []), ...extra()] };
}

/** A raw export between 0.3 and 2.1 MB that ships at a tenth to a fifth of it. */
function file(id: string, name: string, version: number, i: number): ModelFile {
  const raw = Math.round((0.3 + ((i * 7) % 19) / 10.5) * 1e6);
  return {
    model: `${id.replace(/^many-/, '').replaceAll('-', '_')}_v${version}`,
    assetName: name,
    version,
    rawBytes: raw,
    shippedBytes: Math.round(raw * (0.08 + ((i * 3) % 7) / 50)),
  };
}

/** The view with one model per extra asset, every asset skipping the bake. */
export function manyPipeline(view: PipelineView, project: ProjectManifest, state: string | null) {
  if (state !== 'many' || view.project !== 'frostveil' || view.models.length === 0) return view;
  const added = extra().map((a, i) => file(a.id, a.name, a.versions.length, i));
  const assets = (project.assets ?? []).filter(a => a.versions.length > 0);
  return {
    ...view,
    models: [...view.models, ...added],
    bake: view.bake ? { ...view.bake, skipped: assets.map(a => a.id) } : null,
  };
}
