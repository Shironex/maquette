import type {
  Asset,
  AssetKind,
  ConceptJob,
  Direction,
  ImportedConcepts,
  MovedDirection,
  RefImage,
  SentObjects,
  SentPicks,
} from '../client';
import { clock } from '@/lib/clock';
import type { FakeLabOptions } from './lab';
import { objectRef } from './lab-fixtures';
import { finished, refMayReplace, slug } from './lab-helpers';

interface LabSendsDeps {
  options: FakeLabOptions;
  directions: Map<string, Direction[]>;
  refs: Map<string, RefImage[]>;
  jobs: () => ConceptJob[];
  list: (project: string) => Direction[];
  findDirection: (project: string, id: string) => Direction;
  saveDirection: (project: string, next: Direction) => Direction;
}

/** The commands that carry a direction's pictures onward: to Refs, as objects, or to another project. */
export function createLabSends({
  options,
  directions,
  refs,
  jobs,
  list,
  findDirection,
  saveDirection,
}: LabSendsDeps) {
  const { fail } = options;

  /** Mirrors `concepts::send_object_picks`: one asset per pick, named by its title. */
  function sendObjectPicks(project: string, id: string, kind: AssetKind | null): SentObjects {
    const direction = findDirection(project, id);
    const picks = direction.images.filter(i => i.picked);
    if (picks.length === 0) fail('invalid', 'Pick at least one object to send to Refs.');
    const dir = options.projectDir(project);
    const manifest = options.project(project);
    let assets = [...(manifest.assets ?? [])];
    const at = clock.now().toISOString();
    const sent: SentObjects['assets'] = [];
    const copied: string[] = [];
    const images = direction.images.map(image => {
      if (!image.picked) return image;
      const title = image.title ?? `${direction.name} ${image.index}`;
      const ref = image.refFile ?? objectRef(title);
      options.files?.alias(`${dir}/${ref}`, `${dir}/${image.file}`);
      if (!image.refFile) copied.push(ref);
      const assetId = slug(title);
      const known = assets.find(a => a.id === assetId);
      const asset = {
        ...(known ?? {
          id: assetId,
          name: title,
          kind: kind === 'character' ? ('character' as const) : ('prop' as const),
          stage: 'refs' as const,
          status: 'queued' as const,
          versions: [],
          createdAt: at,
        }),
        stage: known && known.stage !== 'concept' ? known.stage : ('refs' as const),
        thumbnail: known && !refMayReplace(known.thumbnail) ? known.thumbnail : ref,
        updatedAt: at,
      };
      assets = known ? assets.map(a => (a.id === assetId ? asset : a)) : [...assets, asset];
      sent.push({ asset, refs: [ref] });
      return { ...image, refFile: ref };
    });
    const next = saveDirection(project, { ...direction, updatedAt: at, images });
    const known = refs.get(project) ?? [];
    const files = sent.flatMap(s => s.refs);
    refs.set(project, [
      ...known.filter(r => !files.includes(r.file)),
      ...files.map(file => ({ file, name: file.slice(5), bytes: 1.2e6 })),
    ]);
    const saved = options.saveProject({ ...manifest, assets });
    return { direction: next, assets: sent, copied, project: saved };
  }

  /** Mirrors `concepts::import::import_images`. */
  function importConcepts(project: string, paths: string[]): ImportedConcepts {
    if (paths.length === 0) fail('invalid', 'Choose at least one image.');
    // The core reads headers; here a known other extension stands in for a bad one.
    for (const path of paths) {
      const name = path.slice(path.lastIndexOf('/') + 1);
      if (/\.[^.]+$/.test(name) && !/\.(png|jpe?g|webp)$/i.test(name))
        fail('invalid', `${name} is not a PNG, JPEG or WebP image.`);
    }
    const stem = (path: string) => path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]*$/, '');
    const words = stem(paths[0]!).replace(/[_-]/g, ' ').trim();
    const name = words ? words[0]!.toUpperCase() + words.slice(1) : 'Dropped images';
    const id = slug(name);
    const at = clock.now().toISOString();
    const dir = options.projectDir(project);
    const found = list(project).find(d => d.id === id);
    const start = Math.max(0, ...(found?.images ?? []).map(i => i.index)) + 1;
    const added = paths.map((_, n) => {
      const index = start + n;
      return `concepts/${id}/${id}_${String(index).padStart(2, '0')}.png`;
    });
    paths.forEach((path, n) => options.files?.alias(`${dir}/${added[n]}`, path));
    const direction = saveDirection(project, {
      id,
      name,
      prompt: '',
      refs: [],
      aspect: '1:1',
      createdAt: at,
      source: null,
      kind: null,
      styleOff: true,
      styleRevision: null,
      refsJob: null,
      ...found,
      updatedAt: at,
      images: [
        ...(found?.images ?? []),
        ...added.map((file, n) => ({
          file,
          index: start + n,
          width: 1024,
          height: 1024,
          starred: false,
          picked: false,
          refFile: null,
          createdAt: at,
          title: null,
        })),
      ],
    });
    const manifest = options.project(project);
    const known = manifest.assets?.find(a => a.id === id);
    const asset: Asset = known
      ? { ...known, thumbnail: known.thumbnail ?? added[0]!, updatedAt: at }
      : {
          id,
          name,
          kind: 'prop',
          stage: 'concept',
          status: 'queued',
          versions: [],
          thumbnail: added[0]!,
          createdAt: at,
          updatedAt: at,
        };
    const saved = options.saveProject({
      ...manifest,
      assets: known
        ? (manifest.assets ?? []).map(a => (a.id === id ? asset : a))
        : [...(manifest.assets ?? []), asset],
    });
    return { direction, asset, added, project: saved };
  }

  function sendPicks(project: string, id: string, kind: AssetKind): SentPicks {
    const direction = findDirection(project, id);
    const picks = direction.images.filter(i => i.picked);
    if (picks.length === 0) fail('invalid', 'Pick at least one concept to send to Refs.');
    const dir = options.projectDir(project);
    const copied = picks.map(i => `refs/${i.file.slice(i.file.lastIndexOf('/') + 1)}`);
    picks.forEach((image, n) =>
      options.files?.alias(`${dir}/${copied[n]}`, `${dir}/${image.file}`)
    );
    const next = saveDirection(project, {
      ...direction,
      updatedAt: clock.now().toISOString(),
      images: direction.images.map(i =>
        i.picked ? { ...i, refFile: `refs/${i.file.slice(i.file.lastIndexOf('/') + 1)}` } : i
      ),
    });
    const known = refs.get(project) ?? [];
    refs.set(project, [
      ...known.filter(r => !copied.includes(r.file)),
      ...copied.map(file => ({ file, name: file.slice(5), bytes: 2.2e6 })),
    ]);
    const manifest = options.project(project);
    const at = clock.now().toISOString();
    const assets = [...(manifest.assets ?? [])];
    const assetId = slug(direction.name);
    const order = ['concept', 'refs', 'tripo', 'blender', 'web', 'ship'];
    let asset = assets.find(a => a.id === assetId);
    // A card images were dropped for only guessed its kind.
    if (asset?.stage === 'concept' && asset.thumbnail?.startsWith('concepts/'))
      asset = { ...asset, kind };
    if (asset && order.indexOf(asset.stage) < 1) asset = { ...asset, stage: 'refs', updatedAt: at };
    if (!asset) {
      asset = {
        id: assetId,
        name: direction.name,
        kind,
        stage: 'refs',
        status: 'queued',
        versions: [],
        thumbnail: null,
        createdAt: at,
        updatedAt: at,
      };
    }
    // The card shows the first pick until a model render replaces it.
    const first = copied[0];
    if (first && refMayReplace(asset.thumbnail)) asset = { ...asset, thumbnail: first };
    const placed = asset;
    const saved = options.saveProject({
      ...manifest,
      assets: assets.some(a => a.id === assetId)
        ? assets.map(a => (a.id === assetId ? placed : a))
        : [...assets, placed],
    });
    return { direction: next, asset: placed, copied, project: saved };
  }

  /** Mirrors `concepts::move_direction`: the folder, its refs and, when free, its card. */
  function moveDirection(from: string, id: string, to: string): MovedDirection {
    if (from === to) fail('invalid', 'The direction is already in this project.');
    const fromProject = options.project(from);
    options.project(to);
    const direction = findDirection(from, id);
    if (jobs().some(j => j.project === from && j.direction === id && !finished(j)))
      fail(
        'invalid',
        'Images of this direction are still generating. Move it once they are done or cancelled.'
      );
    let newId = direction.id;
    for (let n = 2; list(to).some(d => d.id === newId); n += 1) newId = `${direction.id}-${n}`;
    const fromDir = options.projectDir(from);
    const toDir = options.projectDir(to);
    const moved: string[] = [];
    const images = direction.images.map(image => {
      const file = image.file.replace(`concepts/${direction.id}/`, `concepts/${newId}/`);
      options.files?.alias(`${toDir}/${file}`, `${fromDir}/${image.file}`);
      if (image.refFile) {
        options.files?.alias(`${toDir}/${image.refFile}`, `${fromDir}/${image.refFile}`);
        moved.push(image.refFile);
      }
      return { ...image, file };
    });
    for (const ref of direction.refs.filter(r => r.startsWith('refs/')))
      options.files?.alias(`${toDir}/${ref}`, `${fromDir}/${ref}`);
    const next: Direction = {
      ...direction,
      id: newId,
      images,
      updatedAt: clock.now().toISOString(),
    };
    directions.set(
      from,
      list(from).filter(d => d.id !== direction.id)
    );
    directions.set(to, [next, ...list(to)]);
    const carried = [...new Set([...moved, ...direction.refs.filter(r => r.startsWith('refs/'))])];
    const toRefs = refs.get(to) ?? [];
    refs.set(to, [
      ...toRefs.filter(r => !carried.includes(r.file)),
      ...carried.map(file => ({ file, name: file.slice(5), bytes: 2.2e6 })),
    ]);
    const asset = (fromProject.assets ?? []).find(a => a.id === slug(direction.name));
    const takes =
      asset && asset.versions.length === 0 && !options.assetBusy?.(from, asset.id) ? asset : null;
    if (takes) {
      refs.set(
        from,
        (refs.get(from) ?? []).filter(r => !moved.includes(r.file))
      );
      options.saveProject({
        ...fromProject,
        assets: (fromProject.assets ?? []).filter(a => a.id !== takes.id),
      });
      const target = options.project(to);
      options.saveProject({
        ...target,
        assets: [
          ...(target.assets ?? []).filter(a => a.id !== takes.id),
          { ...takes, updatedAt: clock.now().toISOString() },
        ],
      });
    }
    return {
      direction: next,
      from: options.project(from),
      to: options.project(to),
      asset: takes,
      refs: moved,
    };
  }

  return { sendPicks, sendObjectPicks, moveDirection, importConcepts };
}
