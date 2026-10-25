import type { NewAsset, ProjectManifest, Stage } from '../client';
import { clock } from '@/lib/clock';
import { DEFAULT_ROOT } from './fixtures';
import { expand, fail, slugify, type Args, type BackendOptions } from './backend-options';

interface AssetContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  root: () => string | null;
  files: BackendOptions['files'];
}

/** The asset commands: add, move, rename and copy between projects. */
export function createAssetHandlers({
  project,
  saveProject,
  root,
  files,
}: AssetContext): Record<string, (args: Args) => unknown> {
  return {
    asset_add: ({ project: id, asset }) => {
      const p = project(String(id));
      const next = asset as NewAsset;
      const name = next.name.trim();
      if (!name) fail('invalid', 'Give the asset a name.');
      let assetId = slugify(name);
      while ((p.assets ?? []).some(a => a.id === assetId)) assetId = `${assetId}-2`;
      const at = clock.now().toISOString();
      return saveProject({
        ...p,
        assets: [
          ...(p.assets ?? []),
          {
            id: assetId,
            name,
            kind: next.kind,
            stage: next.stage,
            status: 'queued',
            versions: [],
            thumbnail: null,
            createdAt: at,
            updatedAt: at,
          },
        ],
      });
    },
    asset_move: ({ project: id, id: assetId, stage }) => {
      const p = project(String(id));
      if (!(p.assets ?? []).some(a => a.id === assetId))
        fail('not-found', `Could not find the asset ${String(assetId)}.`);
      const at = clock.now().toISOString();
      return saveProject({
        ...p,
        assets: (p.assets ?? []).map(a =>
          a.id === assetId ? { ...a, stage: stage as Stage, updatedAt: at } : a
        ),
      });
    },
    asset_rename: ({ project: id, id: assetId, name }) => {
      const p = project(String(id));
      const next = String(name).trim();
      if (!next) fail('invalid', 'Give the asset a name.');
      if (next.length > 80) fail('invalid', 'Keep the asset name to 80 characters or fewer.');
      const assets = p.assets ?? [];
      if (!assets.some(a => a.id === assetId))
        fail('not-found', `Could not find the asset ${String(assetId)}.`);
      const slug = slugify(next);
      if (assets.some(a => a.id !== assetId && (a.id === slug || slugify(a.name) === slug)))
        fail('invalid', `Another asset is already called ${next}.`);
      const at = clock.now().toISOString();
      return saveProject({
        ...p,
        assets: assets.map(a => (a.id === assetId ? { ...a, name: next, updatedAt: at } : a)),
      });
    },
    asset_open_in_blender: () => fail('unavailable', 'Mock mode does not open Blender.'),
    asset_copy_to_project: ({ project: id, id: assetId, to }) => {
      if (id === to) fail('invalid', 'Pick another project to copy the asset to.');
      const from = project(String(id));
      const target = project(String(to));
      const source = (from.assets ?? []).find(a => a.id === assetId);
      if (!source) fail('not-found', `Could not find the asset ${String(assetId)}.`);
      const taken = (name: string) =>
        (target.assets ?? []).some(
          a => a.id === slugify(name) || slugify(a.name) === slugify(name)
        );
      let name = source.name;
      for (let n = 2; taken(name); n++) name = `${source.name} ${n}`;
      const newId = slugify(name);
      const key = newId.replace(/-/g, '_');
      const oldKey = source.id.replace(/-/g, '_');
      const dir = (pid: string) => `${root() ?? expand(DEFAULT_ROOT)}/projects/${pid}`;
      const versions = source.versions.map(v => {
        const file = `models/${key}_v${v.version}.glb`;
        files?.alias(`${dir(target.id)}/${file}`, `${dir(from.id)}/${v.file}`);
        return { ...v, file };
      });
      const thumbnail = source.thumbnail?.replace(oldKey, key) ?? null;
      if (source.thumbnail && thumbnail) {
        files?.alias(`${dir(target.id)}/${thumbnail}`, `${dir(from.id)}/${source.thumbnail}`);
      }
      const at = clock.now().toISOString();
      const asset = {
        ...source,
        id: newId,
        name,
        versions,
        thumbnail,
        createdAt: at,
        updatedAt: at,
      };
      const next = saveProject({ ...target, assets: [...(target.assets ?? []), asset] });
      return { asset, to: next, refs: [] };
    },
  };
}
