import type {
  AcceptRequest,
  AssetKind,
  InboxItem,
  ModelPrep,
  ProjectManifest,
  Stage,
} from '../client';
import { clock } from '@/lib/clock';
import { DEFAULT_ROOT } from './fixtures';
import { expand, fail, sleep, slugify, type Args, type BackendOptions } from './backend-options';

interface InboxContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  root: () => string | null;
  inbox: { get: () => InboxItem[]; set: (next: InboxItem[]) => void };
  probeDelay: number;
  emit: BackendOptions['emit'];
  files: BackendOptions['files'];
  /** Real size and fix-ups: checked before the move, saved after it. */
  prepare: {
    check: (entry: ModelPrep, what: string) => void;
    saveEntry: (project: string, asset: string, entry: ModelPrep) => void;
    /** Remembers what the accepted file measures, for the prepare views. */
    remember: (file: string, item: InboxItem) => void;
  };
}

/** The inbox commands, plus the thumbnail the viewer stores for an item. */
export function createInboxHandlers({
  project,
  saveProject,
  root,
  inbox,
  probeDelay,
  emit,
  files,
  prepare,
}: InboxContext): Record<string, (args: Args) => unknown> {
  const kinds = new Map<string, AssetKind>();
  return {
    inbox_list: () => ({
      dir: '~/Maquette/inbox',
      path: `${expand(DEFAULT_ROOT)}/inbox`,
      watching: Boolean(root()),
      items: inbox.get().map(i => ({
        ...i,
        kind: kinds.get(i.id) ?? i.kind,
        kindGuessed: !kinds.has(i.id) && !i.kindFromSend,
      })),
    }),
    inbox_reprocess: async ({ id }) => {
      await sleep(probeDelay / 2);
      const item = inbox.get().find(i => i.id === id);
      if (!item) fail('not-found', `${String(id)} in the inbox`);
      return item;
    },
    inbox_set_kind: ({ id, kind }) => {
      const item = inbox.get().find(i => i.id === id);
      if (!item) fail('not-found', `${String(id)} in the inbox`);
      kinds.set(item.id, kind as AssetKind);
      return { ...item, kind: kind as AssetKind, kindGuessed: false, kindFromSend: false };
    },
    inbox_accept: ({ request }) => {
      const req = request as AcceptRequest;
      const item = inbox.get().find(i => i.id === req.item);
      if (!item) fail('not-found', `${req.item} in the inbox`);
      if (req.prep) prepare.check(req.prep, 'The real size and fix-ups');
      const p = project(req.project);
      const assets = [...(p.assets ?? [])];
      const assetId = req.asset ?? slugify(item.name);
      let target = assets.find(a => a.id === assetId);
      const at = clock.now().toISOString();
      if (!target) {
        target = {
          id: assetId,
          name: item.name,
          kind: kinds.get(item.id) ?? item.kind,
          stage: 'tripo',
          status: 'queued',
          versions: [],
          thumbnail: null,
          createdAt: at,
          updatedAt: at,
        };
        assets.push(target);
      }
      const number = target.versions.length + 1;
      const version = {
        version: number,
        file: `models/${assetId.replace(/-/g, '_')}_v${number}.glb`,
        createdAt: at,
        stats: item.report
          ? {
              triangles: item.report.triangles,
              bytes: item.report.bytes,
              textures: item.report.textures.length,
              maxTexture: item.report.maxTexture,
              joints: item.report.joints,
              animations: item.report.animations.length,
            }
          : null,
      };
      // The service renders a thumbnail for the new version; mock mode reuses the inbox one.
      let thumbnail = target.thumbnail;
      if (item.thumbnail) {
        thumbnail = `thumbs/${assetId.replace(/-/g, '_')}_v${number}.png`;
        files?.alias(`${expand(DEFAULT_ROOT)}/projects/${p.id}/${thumbnail}`, item.thumbnail);
      }
      const updated = {
        ...target,
        thumbnail,
        versions: [...target.versions, version],
        stage: 'blender' as Stage,
        status: 'review' as const,
        updatedAt: at,
      };
      const next = saveProject({ ...p, assets: assets.map(a => (a.id === assetId ? updated : a)) });
      inbox.set(inbox.get().filter(i => i.id !== item.id));
      prepare.remember(version.file, item);
      if (req.prep) prepare.saveEntry(p.id, assetId, req.prep);
      emit?.({ name: 'inbox' });
      return {
        asset: updated,
        version,
        file: `${expand(DEFAULT_ROOT)}/projects/${p.id}/${version.file}`,
        project: next,
        note: null,
      };
    },
    inbox_open_in_blender: () => fail('unavailable', 'Mock mode does not open Blender.'),
    thumbnail_store: ({ file, png }) => {
      const item = inbox.get().find(i => i.file === file);
      if (!item) fail('not-found', `${String(file)} in the inbox`);
      const path = `${expand(DEFAULT_ROOT)}/cache/thumbs/${item.id}.png`;
      files?.store(path, Uint8Array.from(png as number[]));
      inbox.set(
        inbox
          .get()
          .map(i =>
            i.id === item.id ? { ...i, thumbnail: path, thumbnailState: 'ready' as const } : i
          )
      );
      return path;
    },
  };
}
