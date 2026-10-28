/**
 * The refs sheet in mock mode: an asset's views, the Lab jobs that draw
 * the missing ones and clean a ref, and the weak ref warnings, mirroring
 * `maquette_core::refs`. `?lab=refs` tags the guardian statues: a front,
 * a busy crop and a small detail as other, so the sheet shows 1 of 4
 * views. The front crops its object, so a send of it warns.
 */
import type {
  AssetRef,
  ConceptJob,
  Direction,
  LabView,
  NewCleanRef,
  NewOtherViews,
  ProjectManifest,
  RefImage,
  RefFix,
  RefView,
  RefWarning,
  RefsView,
  WarningsItem,
} from '../client';
import { clock } from '@/lib/clock';
import { DRAWN, VIEW_LABEL, assign, emptyViews, sendOrder, sentView } from '@/state/refs';
import type { FakeLabOptions } from './lab';
import { slug } from './lab-helpers';
import { refImages, refsOf } from './provider-refs';

type Args = Record<string, unknown>;

const GUARDIAN = 'guardian-statues';
const FRONT = 'refs/guardian_statues_front.png';
const CROP = 'refs/guardian_statues_crop.png';
const DETAIL = 'refs/guardian_statues_detail.png';

/** The scenario's extra refs, served from the bundled concepts. */
export const REFS_SCENARIO_FILES: Record<string, string> = {
  [CROP]: 'concepts/veil-gate-kit-a.webp',
  [DETAIL]: 'concepts/veil-gate-step-ring.webp',
};

type Weak = 'busy' | 'small' | 'cropped';

/** What the core's pixel checks find in the scenario's refs. */
const WEAK: Record<string, Weak> = { [FRONT]: 'cropped', [CROP]: 'busy', [DETAIL]: 'small' };

/** The core's copy for each check; `who` is the view an image goes as, else its name. */
function weakWarning(weak: Weak, who: string, name: string) {
  const fixes: RefFix[] = ['clean-background', 'leave-out'];
  if (weak === 'small')
    return {
      kind: 'small' as const,
      title: `${who} is small: 384 × 384 px`,
      detail: 'Below 512 px a model reads poorly. Draw it again at 1024 or leave it out.',
      fixes: ['leave-out' as const],
    };
  if (weak === 'busy')
    return {
      kind: 'busy-background' as const,
      title: `Busy background in ${name}`,
      detail:
        'Clutter around the object can end up in the model. Clean background keeps the object and clears the rest.',
      fixes,
    };
  return {
    kind: 'cropped' as const,
    title: `${who} crops the object`,
    detail: 'The object touches the bottom edge, so the model can end flat there.',
    fixes,
  };
}

export interface RefsLabParts {
  options: FakeLabOptions;
  refs: Map<string, RefImage[]>;
  list: (project: string) => Direction[];
  saveDirection: (project: string, next: Direction) => Direction;
  enqueue: (project: string, direction: Direction, titles: (string | null)[]) => LabView;
  jobs: () => ConceptJob[];
  /** The pool image a direction's jobs come back as, so candidates look like the asset. */
  standIns: Map<string, string>;
}

export function createRefsHandlers(parts: RefsLabParts): Record<string, (args: Args) => unknown> {
  const { options, refs, list, saveDirection } = parts;
  const fail: (code: string, message: string) => never = options.fail;
  const images = (project: string) => refImages(refs.get(project) ?? []);
  const sentFor = (project: string, name: string) =>
    list(project)
      .filter(d => !d.refsJob && d.name.trim().toLowerCase() === name.trim().toLowerCase())
      .flatMap(d => d.images.flatMap(i => (i.refFile ? [i.refFile] : [])));
  const assetOf = (p: ProjectManifest, id: string) =>
    (p.assets ?? []).find(a => a.id === id) ?? fail('not-found', `Could not find the asset ${id}.`);
  const current = (p: ProjectManifest, id: string) =>
    refsOf({ sentFor }, p, assetOf(p, id), images(p.id));

  if (options.state === 'refs') seed();

  function seed() {
    const known = refs.get('frostveil') ?? [];
    const extra = Object.keys(REFS_SCENARIO_FILES).map(file => ({
      file,
      name: file.slice('refs/'.length),
      bytes: file === DETAIL ? 1.8e5 : 2.4e6,
    }));
    refs.set('frostveil', [...known, ...extra]);
    const p = options.project('frostveil');
    const tags: AssetRef[] = [
      { file: FRONT, view: 'front' },
      { file: CROP, view: 'other' },
      { file: DETAIL, view: 'other' },
    ];
    const assets = (p.assets ?? []).map(a => (a.id === GUARDIAN ? { ...a, refs: tags } : a));
    options.saveProject({ ...p, assets });
  }

  function view(project: string, asset: string): RefsView {
    const p = options.project(project);
    const found = assetOf(p, asset);
    const directions = list(project).filter(d => d.refsJob?.asset === asset);
    return {
      project: p,
      projectDir: options.projectDir(project),
      asset: found,
      refs: current(p, asset),
      tagged: (found.refs ?? []).length > 0,
      images: images(project),
      directions,
      jobs: parts
        .jobs()
        .filter(j => j.project === project && directions.some(d => d.id === j.direction)),
    };
  }

  function save(project: string, asset: string, next: AssetRef[]) {
    const p = options.project(project);
    const at = clock.now().toISOString();
    const assets = (p.assets ?? []).map(a =>
      a.id === asset ? { ...a, refs: next, updatedAt: at } : a
    );
    options.saveProject({ ...p, assets });
  }

  function setViews(project: string, asset: string, next: AssetRef[]) {
    const known = images(project);
    for (const r of next) {
      if (!known.some(i => i.file === r.file)) fail('not-found', `the ref ${r.file}`);
      const taken = r.view !== 'other' && next.find(o => o !== r && o.view === r.view);
      if (taken)
        fail(
          'invalid',
          `${taken.file} and ${r.file} are both tagged ${VIEW_LABEL[r.view]}. A view holds one ref.`
        );
    }
    save(project, asset, next);
    return view(project, asset);
  }

  /** A refs direction for the asset, numbered when another asset holds the name. */
  function direction(project: string, asset: string, kind: 'views' | 'clean', source: string) {
    const name = `${assetOf(options.project(project), asset).name} ${kind}`;
    const id = slug(name);
    const existing = list(project).find(d => d.id === id);
    const at = clock.now().toISOString();
    return saveDirection(project, {
      id,
      name,
      prompt:
        kind === 'views'
          ? 'The same object from the other sides, true to the front.'
          : 'The same object, the clutter around it cleared.',
      refs: [source],
      aspect: '1:1',
      images: existing?.images ?? [],
      createdAt: existing?.createdAt ?? at,
      updatedAt: at,
      source,
      kind: null,
      styleRevision: null,
      styleOff: false,
      refsJob: { asset, kind },
    });
  }

  function makeViews(request: NewOtherViews) {
    const p = options.project(request.project);
    const refsNow = current(p, request.asset);
    const front = refsNow.find(r => r.view === 'front');
    if (!front) fail('invalid', 'Tag a front first: the other views are drawn from it.');
    if (request.count < 1 || request.count > 4) fail('invalid', 'Ask for 1 to 4 images per view.');
    const asked = request.views ?? [];
    if (asked.some(v => !(DRAWN as readonly RefView[]).includes(v)))
      fail('invalid', 'A drawn view is left, back or right; the front stays as it is.');
    const views = asked.length > 0 ? asked : emptyViews(refsNow);
    if (views.length === 0) fail('invalid', 'Every view is filled. Name a view to draw it again.');
    const d = direction(request.project, request.asset, 'views', front.file);
    if (request.asset === GUARDIAN) parts.standIns.set(d.id, 'veil-gate-hooded-guardian.webp');
    parts.enqueue(
      request.project,
      d,
      views.flatMap(v => Array<string>(request.count).fill(v))
    );
    return view(request.project, request.asset);
  }

  function clean(request: NewCleanRef) {
    const p = options.project(request.project);
    if (!current(p, request.asset).some(r => r.file === request.file))
      fail('invalid', `${request.file} is not a ref of this asset.`);
    const d = direction(request.project, request.asset, 'clean', request.file);
    if (request.asset === GUARDIAN) parts.standIns.set(d.id, 'veil-gate-hooded-guardian.webp');
    parts.enqueue(request.project, d, [request.file]);
    return view(request.project, request.asset);
  }

  /** "Use as Left" or "Use the clean copy", as `refs::use_view_image` and `use_clean_copy`. */
  function take(project: string, asset: string, file: string, as: RefView | null) {
    const dir = options.projectDir(project);
    const d = list(project).find(x => file.startsWith(`concepts/${x.id}/`));
    const image = d?.images.find(i => i.file === file);
    if (!d?.refsJob || d.refsJob.asset !== asset || !image)
      fail('invalid', `${file} was not made for the refs of this asset.`);
    const p = options.project(project);
    const key = asset.replace(/-/g, '_');
    const kept = image.refFile;
    const stem = image.title?.slice(image.title.lastIndexOf('/') + 1).replace(/\.\w+$/, '');
    const copy =
      kept ??
      (d.refsJob.kind === 'views' ? `refs/${key}_${image.title}.png` : `refs/${stem}_clean.png`);
    options.files?.alias(`${dir}/${copy}`, `${dir}/${file}`);
    refs.set(project, [
      ...(refs.get(project) ?? []).filter(r => r.file !== copy),
      { file: copy, name: copy.slice('refs/'.length), bytes: 2.2e6 },
    ]);
    saveDirection(project, {
      ...d,
      images: d.images.map(i => (i.file === file ? { ...i, refFile: copy, picked: true } : i)),
    });
    let next = current(p, asset);
    if (d.refsJob.kind === 'views') {
      const slot = as ?? (DRAWN as readonly string[]).find(v => v === image.title);
      if (!slot) fail('invalid', 'Say which view this image shows.');
      if (slot === 'front')
        fail('invalid', 'The front stays as it is; a drawn view fills left, back or right.');
      next = assign(next, copy, slot as RefView);
    } else {
      const original = image.title ?? '';
      const was = next.find(r => r.file === original)?.view ?? 'other';
      refs.set(
        project,
        (refs.get(project) ?? []).filter(r => r.file !== original)
      );
      next = [
        ...next.filter(r => r.file !== original && r.file !== copy),
        { file: copy, view: was },
      ];
    }
    save(project, asset, next);
    return view(project, asset);
  }

  function warnings(project: string, items: WarningsItem[]): RefWarning[][] {
    const p = options.project(project);
    return items.map(item => {
      const asset = assetOf(p, item.asset);
      const sent = sendOrder(asset, item.images);
      const out: RefWarning[] = sent.flatMap((file): RefWarning[] => {
        const weak = WEAK[file];
        if (!weak) return [];
        const view = sent.length > 1 ? sentView(asset, item.images, file) : null;
        const name = file.slice(file.lastIndexOf('/') + 1);
        return [{ file, view, ...weakWarning(weak, view ? VIEW_LABEL[view] : name, name) }];
      });
      if (sent.length === 1 && item.multiView)
        out.push({
          kind: 'one-view',
          file: null,
          view: null,
          title: 'One view only: the sides and back are guessed',
          detail:
            'Multi-view is on, but only one image goes. Make the other views to send all four.',
          fixes: ['make-views'],
        });
      return out;
    });
  }

  return {
    refs_get: ({ project, asset }) => view(String(project), String(asset)),
    refs_set_views: ({ project, asset, refs: next }) =>
      setViews(String(project), String(asset), next as AssetRef[]),
    refs_make_views: ({ request }) => makeViews(request as NewOtherViews),
    refs_clean: ({ request }) => clean(request as NewCleanRef),
    refs_use: ({ project, asset, file, view: as }) =>
      take(String(project), String(asset), String(file), (as as RefView | null) ?? null),
    refs_warnings: ({ project, items }) => warnings(String(project), items as WarningsItem[]),
  };
}
