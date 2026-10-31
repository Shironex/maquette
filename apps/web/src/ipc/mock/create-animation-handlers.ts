/**
 * The animation library in mock mode: clips kept in memory, an import
 * that reads file names the way the core reads FBX files and pack zips,
 * and Save clips to the library from the models the mock bundles.
 * `?library=demo` opens with the clips of `animation-fixtures.ts`.
 * Nothing here spends credits, as in the app.
 */
import type {
  AnimationClip,
  AnimationsView,
  ClipsFrom,
  ImportFile,
  ImportPlan,
  ImportRequest,
  Imported,
  PlanClip,
  ProjectManifest,
  SavePlan,
  SaveClips,
  Saved,
  SourceRig,
} from '../client';
import { clock } from '@/lib/clock';
import { fileUrl, registerMockFile } from '../files';
import { fail, type Args } from './backend-options';
import { HOME } from './fixtures';
import { PACK_CLIPS, animationClips, mockClip } from './animation-fixtures';
import { MOTION_FIXTURES } from './motion-fixtures';
import { createRetargetHandlers } from './create-retarget-handlers';

interface AnimationContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  alias: (path: string, from: string) => void;
  root: () => string;
  state: string | null;
}

/** Where `install.ts` serves the bundled clip, for every clip to play. */
export const clipSource = (root: string) => `${root}/library/animations/.bundled/clip.glb`;

/** The FBX files and zips Add files picks in mock mode. */
const PICKED = [
  'Pro Sword and Shield Pack.zip',
  'Pro Magic Pack.zip',
  'Standing Dodge Forward.fbx',
  'heroine_cast_take3.fbx',
].map(f => `${HOME}/Downloads/${f}`);

const base = (path: string) => path.split(/[\\/]/).pop() ?? path;
const stem = (file: string) => base(file).replace(/\.[^.]+$/, '');

const TAGS: readonly [string, RegExp][] = [
  ['dodge', /dodge|roll/i],
  ['combat', /sword|shield|slash|block|kick|dodge|attack|lunge/i],
  ['magic', /magic|spell|cast|heal/i],
  ['locomotion', /run|walk|turn|jump|roll/i],
  ['idle', /idle/i],
  ['hit react', /impact|hit|stagger/i],
];

const tagsFor = (name: string) => TAGS.filter(([, re]) => re.test(name)).map(([t]) => t);

function rigOf(names: readonly string[]): SourceRig {
  if (names.some(n => n.startsWith('mixamorig'))) return 'mixamo';
  return names.includes('Hip') && names.includes('L_Thigh') ? 'tripo' : 'custom';
}

export function createAnimationHandlers(ctx: AnimationContext) {
  let clips: AnimationClip[] = ctx.state === 'demo' ? animationClips() : [];
  const plans = new Map<string, ImportPlan>();
  let reads = 0;
  const dir = () => `${ctx.root()}/library/animations`;
  const fileOf = (id: string) => `${dir()}/${id}/clip.glb`;
  const serve = (id: string) => {
    const url = fileUrl(clipSource(ctx.root()));
    if (url) registerMockFile(fileOf(id), url);
  };
  for (const c of clips) serve(c.id);
  const retarget = createRetargetHandlers({ ...ctx, clips: () => clips });

  const view = (): AnimationsView => ({
    dir: dir(),
    items: clips.map(clip => ({ clip, file: fileOf(clip.id), uses: retarget.uses(clip) })),
    diskBytes: clips.reduce((sum, c) => sum + (c.bytes ?? 0), 0),
  });

  function planClip(name: string, pack: string, file: string, key: string): PlanClip {
    const rig: SourceRig = /heroine|custom/i.test(file) ? 'custom' : 'mixamo';
    const draft = mockClip({ name, pack, tags: tagsFor(name), frames: 40, kb: 48, rig, at: '' });
    const same = clips.find(c => c.name === name);
    return {
      key,
      name,
      file,
      pack,
      tags: draft.tags,
      rig,
      bones: draft.rig.bones.length,
      frames: rig === 'custom' ? 40 : 38,
      fps: rig === 'custom' ? 24 : 30,
      length: rig === 'custom' ? 40 / 24 : 38 / 30,
      rootMotion: /dodge|run|roll|walk/i.test(name) ? { metres: 0.92, direction: 'left' } : null,
      loops: /idle|run|walk/i.test(name),
      bytes: draft.bytes,
      status: same ? 'duplicate' : 'new',
      matched: same ? { id: same.id, name: same.name, pack: same.pack } : null,
    };
  }

  function readFile(path: string, index: number, singles: string): ImportFile {
    const name = base(path);
    const lower = name.toLowerCase();
    const file = (over: Partial<ImportFile>): ImportFile => ({
      name,
      archive: false,
      bytes: 1_200_000,
      fbx: 1,
      pack: singles,
      rig: 'mixamo',
      bones: 22,
      prefix: 'mixamorig:',
      fps: 30,
      skinned: 1,
      clips: [],
      leftOut: [],
      ...over,
    });
    if (lower.endsWith('.zip')) {
      const pack = stem(name);
      const known = PACK_CLIPS[pack] ?? { clips: [`${pack} 01`, `${pack} 02`], tags: [] };
      const names = known.clips;
      return file({
        archive: true,
        bytes: (names.length + 1) * 2_200_000,
        fbx: names.length + (known.still ? 1 : 0),
        pack,
        skinned: names.length,
        clips: names.map((n, i) => planClip(n, pack, `${n}.fbx`, `${index}/${i}`)),
        leftOut: known.still
          ? [{ file: known.still, reason: 'it holds one pose and no keys' }]
          : [],
      });
    }
    if (!lower.endsWith('.fbx')) {
      return file({
        fbx: 0,
        rig: null,
        bones: 0,
        prefix: null,
        skinned: 0,
        leftOut: [{ file: name, reason: 'it is not an FBX file or a zip of them' }],
      });
    }
    if (/pose/.test(lower)) {
      return file({ leftOut: [{ file: name, reason: 'it holds one pose and no keys' }] });
    }
    const clip = planClip(stem(name).replaceAll('_', ' '), singles, name, `${index}/0`);
    const custom = clip.rig === 'custom';
    return file({
      rig: clip.rig,
      bones: custom ? 58 : 22,
      prefix: custom ? 'heroine_' : 'mixamorig:',
      fps: clip.fps,
      skinned: custom ? 0 : 1,
      clips: [clip],
    });
  }

  function readImport(paths: readonly string[]): ImportPlan {
    const files = paths.map((p, i) => readFile(p, i, 'Singles'));
    const all = files.flatMap(f => f.clips);
    const going = all.filter(c => c.status !== 'duplicate');
    const tags = [...new Set(all.flatMap(c => c.tags))].map(tag => ({
      tag,
      count: all.filter(c => c.tags.includes(tag)).length,
    }));
    reads += 1;
    const plan: ImportPlan = {
      token: `import-${reads}`,
      files,
      readBytes: files.reduce((s, f) => s + (f.bytes ?? 0), 0),
      storedBytes: going.reduce((s, c) => s + (c.bytes ?? 0), 0),
      tags: tags.sort((a, b) => b.count - a.count),
      add: going.length,
      leftOut: files.reduce((s, f) => s + f.leftOut.length, 0),
    };
    plans.set(plan.token, plan);
    return plan;
  }

  function add(clip: AnimationClip): AnimationClip {
    let id = clip.id;
    for (let n = 2; clips.some(c => c.id === id); n += 1) id = `${clip.id}-${n}`;
    const made = { ...clip, id };
    clips = [...clips, made];
    serve(id);
    return made;
  }

  function runImport(request: ImportRequest): Imported {
    const plan =
      plans.get(request.token) ??
      fail('not-found', 'Could not find that import. Add the files again.');
    const at = new Date(clock.nowMs()).toISOString();
    const added: AnimationClip[] = [];
    let skipped = 0;
    for (const f of plan.files) {
      const pack =
        request.packs.find(p => p.file === f.name)?.pack ??
        (f.archive ? f.pack : (request.singles ?? 'Singles'));
      for (const c of f.clips) {
        const keep = request.keepBoth.includes(c.key);
        if (request.skip.includes(c.key) || (c.status === 'duplicate' && !keep)) {
          skipped += 1;
          continue;
        }
        const frames = c.frames;
        added.push(
          add(
            mockClip({
              name: c.name,
              pack,
              tags: c.tags,
              frames,
              fps: c.fps ?? 30,
              root: c.rootMotion ?? undefined,
              loops: c.loops,
              rig: c.rig,
              kb: (c.bytes ?? 0) / 1000,
              at,
              from: c.file,
            })
          )
        );
      }
    }
    plans.delete(request.token);
    return { added, revised: [], skipped, leftOut: plan.leftOut };
  }

  function modelOf(from: ClipsFrom) {
    if (from.inbox)
      return {
        name: stem(from.inbox),
        version: null,
        file: from.inbox,
        key: from.inbox,
        bytes: 5_960_000,
      };
    const p = ctx.project(from.project ?? '');
    const asset =
      (p.assets ?? []).find(a => a.id === from.asset) ??
      fail('not-found', 'Could not find that asset.');
    const latest = asset.versions.at(-1) ?? fail('invalid', `${asset.name} has no model yet.`);
    const key = `${stem(latest.file).replace(/_v\d+$/, '')}.glb`;
    return {
      name: asset.name,
      version: latest.version,
      file: latest.file,
      key,
      bytes: latest.stats?.bytes ?? 0,
    };
  }

  function savePlan(from: ClipsFrom): SavePlan {
    const model = modelOf(from);
    const fixture = MOTION_FIXTURES[model.key];
    if (!fixture?.rig) fail('invalid', `${model.name} has no rig to save clips from.`);
    if (!fixture.clips.length) fail('invalid', `${model.name} has no clips.`);
    const names = fixture.clips.map(c => c.name);
    return {
      from,
      name: model.name,
      version: model.version,
      file: model.file,
      rig: rigOf(fixture.rig.jointNames),
      bones: fixture.rig.joints,
      clips: fixture.clips.map(c => ({
        name: c.name,
        frames: c.frames,
        fps: c.fps,
        rootTravel: c.rootTravel,
        loops: c.loops,
        bytes: c.bytes,
        fromLibrary: clips.find(l => l.name === c.name)?.name ?? null,
      })),
      pack: `${model.name} moves`,
      tags: [...new Set(names.flatMap(tagsFor))],
      modelBytes: Math.max(
        0,
        model.bytes - fixture.clips.reduce((sum, c) => sum + (c.bytes ?? 0), 0)
      ),
    };
  }

  function save(request: SaveClips): Saved {
    const plan = savePlan(request.from);
    const at = new Date(clock.nowMs()).toISOString();
    const out: Saved = { added: [], skipped: [] };
    for (const c of plan.clips.filter(c => request.clips.includes(c.name) && !c.fromLibrary)) {
      if (clips.some(l => l.hash === `${plan.name}/${c.name}`)) {
        out.skipped.push(c.name);
        continue;
      }
      const clip = mockClip({
        name: c.name,
        pack: request.pack,
        tags: request.tags.length ? request.tags : tagsFor(c.name),
        frames: c.frames,
        fps: c.fps ?? 30,
        loops: Boolean(c.loops),
        rig: plan.rig,
        kb: (c.bytes ?? 0) / 1000,
        at,
        from: base(plan.file),
      });
      out.added.push(
        add({
          ...clip,
          hash: `${plan.name}/${c.name}`,
          origin: {
            ...clip.origin,
            source: request.from.inbox ? 'tripo' : 'model',
            archive: null,
            project: request.from.project,
            asset: request.from.asset,
            version: plan.version,
          },
        })
      );
    }
    return out;
  }

  return {
    bakedClips: retarget.bakedClips,
    handlers: {
      ...retarget.handlers,
      animations_get: () => view(),
      dialog_pick_clips: () => PICKED,
      animations_read_import: (args: Args) => readImport(args.paths as string[]),
      animations_import: (args: Args) => runImport(args.request as ImportRequest),
      animations_save_plan: (args: Args) => savePlan(args.from as ClipsFrom),
      animations_save: (args: Args) => save(args.request as SaveClips),
    } as Record<string, (args: Args) => unknown>,
  };
}
