/**
 * Use on model in mock mode: the bone map read from bone names the way the
 * core reads them, settings normalised the same way, a review made from
 * the clip names (a crouch fails on its units, a run's foot slides until
 * its feet are pinned), and a bake that adds the model's next version.
 * `?library=demo` opens with Elsheria having baked a few clips, one of
 * them since fixed.
 */
import { pinOffReview } from './pin-off-review';
import type {
  AnimationClip,
  Baked,
  BonePair,
  ClipReview,
  ClipSetting,
  Clip,
  ClipUse,
  MapChoice,
  ProjectAnimations,
  ProjectManifest,
  RetargetDraft,
  RetargetPlan,
  UseClips,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { MOTION_FIXTURES, type MotionFixture } from './motion-fixtures';
import { HUMANOID, boneIn, defaultName, snake } from './retarget-names';
import { clipInFile, type AssetRecord, type BakedClip } from './retarget-baked';
import { onlyBones, restCompare } from './retarget-rest';

interface RetargetContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  root: () => string;
  clips: () => readonly AnimationClip[];
  alias: (path: string, from: string) => void;
  state: string | null;
}

export function createRetargetHandlers(ctx: RetargetContext) {
  const records = new Map<string, AssetRecord>();
  const drafts = new Map<string, { draft: RetargetDraft; revisions: [string, number, string][] }>();
  let runs = 0;
  const keyOf = (project: string, asset: string) => `${project}/${asset}`;
  const record = (project: string, asset: string) =>
    records.get(keyOf(project, asset)) ?? {
      map: [],
      follow: [],
      baked: [],
      review: null,
      reviewVersion: null,
    };
  const clipOf = (id: string) =>
    ctx.clips().find(c => c.id === id) ?? fail('not-found', `the library clip ${id}`);

  function modelOf(projectId: string, assetId: string) {
    const project = ctx.project(projectId);
    const asset =
      (project.assets ?? []).find(a => a.id === assetId) ??
      fail('not-found', 'Could not find that asset.');
    const latest = asset.versions.at(-1) ?? fail('invalid', `${asset.name} has no model yet.`);
    const fixture = MOTION_FIXTURES[`${latest.file.split('/').pop()?.replace(/_v\d+/, '')}`];
    if (!fixture?.rig) fail('invalid', `${asset.name} has no rig, so clips cannot go on it.`);
    return { project, asset, latest, fixture };
  }

  /** The model side comes from the rig check's own map, as the core reads it. */
  function suggest(
    source: readonly string[],
    rig: NonNullable<MotionFixture['rig']>,
    saved: MapChoice[],
    picked: MapChoice[]
  ) {
    return HUMANOID.map((bone): BonePair => {
      const choice = (list: MapChoice[]) => list.find(c => c.bone === bone);
      const hand = choice(picked) ?? choice(saved);
      const own = rig.map.find(m => m.bone === bone && m.joint);
      const found = own?.joint
        ? { name: own.joint, how: own.how === 'exact' ? ('exact' as const) : ('by-name' as const) }
        : boneIn(rig.jointNames, bone);
      const chosen = hand ? hand.target : (found?.name ?? null);
      const how = !chosen
        ? 'missing'
        : choice(picked)
          ? 'picked'
          : hand
            ? 'saved'
            : (found?.how ?? 'missing');
      return { bone, source: boneIn(source, bone)?.name ?? null, target: chosen, how };
    });
  }

  function normalise(settings: readonly ClipSetting[], taken: readonly string[]) {
    const used = [...taken];
    return settings.map(s => {
      const clip = clipOf(s.clip);
      const last = clip.frames;
      const base = snake(s.name) || defaultName(clip.name, clip.pack);
      let name = base;
      for (let n = 2; used.includes(name); n += 1) name = `${base}_${n}`;
      used.push(name);
      const ok = s.start < s.end && s.end <= last;
      return {
        ...s,
        name,
        start: ok ? s.start : 0,
        end: ok ? s.end : last,
        speed: Math.min(4, Math.max(0.25, s.speed || 1)),
      };
    });
  }

  function plan(
    projectId: string,
    assetId: string,
    ids: string[],
    picked: MapChoice[]
  ): RetargetPlan {
    const { asset, latest, fixture } = modelOf(projectId, assetId);
    const rec = record(projectId, assetId);
    const clips = ids.map(clipOf);
    const source = clips[0]?.rig.bones.map(b => b.name) ?? [];
    const joints = fixture.rig?.jointNames ?? [];
    const map = suggest(source, fixture.rig!, rec.map, picked);
    const replaced = rec.baked.filter(b => ids.includes(b.setting.clip)).map(b => b.setting.name);
    const own = fixture.clips.map(c => c.name).filter(n => !replaced.includes(n));
    const defaults = clips.map(
      (c): ClipSetting =>
        rec.baked.find(b => b.setting.clip === c.id)?.setting ?? {
          clip: c.id,
          name: defaultName(c.name, c.pack),
          start: 0,
          end: c.frames,
          loops: c.loops,
          rootMotion: Boolean(c.rootMotion),
          speed: 1,
        }
    );
    const used = map.map(p => p.target);
    const saved = rec.map.length > 0;
    return {
      project: projectId,
      asset: assetId,
      name: asset.name,
      version: latest.version,
      file: `${ctx.root()}/projects/${projectId}/${latest.file}`,
      joints: joints.length,
      modelClips: fixture.clips.map(c => c.name),
      modelBytes: latest.stats?.bytes ?? 0,
      sourceRig: clips[0]?.rig.kind ?? null,
      sourceBones: source.length,
      map,
      saved,
      needsLook: !saved && map.some(p => !p.bone.endsWith('ToeBase') && (!p.source || !p.target)),
      onlyOnModel: joints.filter(j => !used.includes(j)),
      onlyBones: onlyBones(joints, map, rec.follow),
      rest: restCompare(clips[0], joints, map),
      settings: normalise(defaults, own),
      outdated: rec.baked
        .filter(b => ids.includes(b.setting.clip) && clipOf(b.setting.clip).revision > b.revision)
        .map(b => b.setting.clip),
    };
  }

  function reviewOf(setting: ClipSetting): ClipReview {
    const clip = clipOf(setting.clip);
    const frames = Math.floor((setting.end - setting.start) / (setting.speed ?? 1)) + 1;
    const travel = setting.rootMotion ? (clip.rootMotion?.metres ?? 0) * 0.95 : null;
    const base = {
      clip: clip.id,
      name: setting.name,
      frames,
      travel,
      slideCm: 0.4,
      slideFrames: null,
      unpinnedCm: null,
      pinWorseCm: null,
    };
    if (/\bblock\b/i.test(clip.name)) {
      return pinOffReview({ ...base, status: 'look', note: '' }, setting.pinFeet ?? false);
    }
    if (/crouch/i.test(clip.name)) {
      return {
        ...base,
        status: 'failed',
        note: 'Failed: the hips leave by 44 m, so its keys are likely in the wrong units',
      };
    }
    if (/run\b/i.test(clip.name) && setting.rootMotion && setting.pinFeet) {
      return {
        ...base,
        status: 'good',
        note: 'Feet pinned, the slide is down from 4.2 cm to 0.3 cm',
        slideCm: 0.3,
        unpinnedCm: 4.2,
      };
    }
    if (/run\b/i.test(clip.name) && setting.rootMotion) {
      return {
        ...base,
        status: 'look',
        note: 'The left foot slides 4.2 cm while planted, frames 9 to 14',
        slideCm: 4.2,
        slideFrames: [9, 14],
      };
    }
    const note = travel ? `Looks right, ${travel.toFixed(2)} m root motion` : 'Looks right';
    return { ...base, status: 'good', note };
  }

  function retarget(request: UseClips): RetargetDraft {
    if (!request.settings.length) fail('invalid', 'Choose at least one clip.');
    const p = plan(
      request.project,
      request.asset,
      request.settings.map(s => s.clip),
      request.map
    );
    const replaced = record(request.project, request.asset)
      .baked.filter(b => request.settings.some(s => s.clip === b.setting.clip))
      .map(b => b.setting.name);
    const settings = normalise(
      request.settings,
      p.modelClips.filter(n => !replaced.includes(n))
    );
    if (request.saveMap) {
      const rec = record(request.project, request.asset);
      records.set(keyOf(request.project, request.asset), {
        ...rec,
        map: p.map.map(b => ({ bone: b.bone, target: b.target })),
        follow: [
          ...rec.follow.filter(f => !(request.follow ?? []).some(n => n.bone === f.bone)),
          ...(request.follow ?? []),
        ].filter(f => f.follows),
      });
    }
    runs += 1;
    const clips = settings.map(reviewOf);
    const draft: RetargetDraft = {
      token: `retarget-${runs}`,
      project: request.project,
      asset: request.asset,
      name: p.name,
      fromVersion: p.version,
      review: {
        clips,
        seconds: 4.8 * settings.length,
        driven: p.map.filter(b => b.target).length,
        atRest: p.map.filter(b => !b.target).map(b => b.bone),
        alignedDeg: 47,
        hipsScale: 0.94,
      },
      file: p.file,
      settings: settings.map((s, i) => (clips[i]?.pinWorseCm ? { ...s, pinFeet: false } : s)),
    };
    drafts.set(draft.token, {
      draft,
      revisions: settings.map(s => [s.clip, clipOf(s.clip).revision, clipOf(s.clip).name]),
    });
    return draft;
  }

  function bake(token: string): Baked {
    const found = drafts.get(token) ?? fail('not-found', 'that retarget; run it again');
    const { draft } = found;
    const { project, asset, latest } = modelOf(draft.project, draft.asset);
    const kept = draft.review.clips.filter(c => c.status !== 'failed');
    if (!kept.length) fail('invalid', 'Every clip failed in review, so there is nothing to bake.');
    const number = latest.version + 1;
    const file = latest.file.replace(/_v\d+\.glb$/, `_v${number}.glb`);
    const dir = `${ctx.root()}/projects/${project.id}`;
    ctx.alias(`${dir}/${file}`, `${dir}/${latest.file}`);
    const now = new Date(clock.nowMs()).toISOString();
    const version = { ...latest, version: number, file, createdAt: now };
    const next = ctx.saveProject({
      ...project,
      assets: (project.assets ?? []).map(a =>
        a.id === asset.id ? { ...a, versions: [...a.versions, version], updatedAt: now } : a
      ),
    });
    const rec = record(draft.project, draft.asset);
    const baked: BakedClip[] = kept.map(c => {
      const [, revision, libraryName] = found.revisions.find(r => r[0] === c.clip) ?? [
        c.clip,
        1,
        c.name,
      ];
      return {
        setting: draft.settings.find(s => s.clip === c.clip)!,
        revision,
        libraryName,
        version: number,
        libraryFrames: clipOf(c.clip).frames,
      };
    });
    records.set(keyOf(draft.project, draft.asset), {
      map: rec.map,
      follow: rec.follow,
      baked: [
        ...rec.baked.filter(b => !baked.some(n => n.setting.clip === b.setting.clip)),
        ...baked,
      ],
      review: draft.review,
      reviewVersion: number,
    });
    drafts.delete(token);
    return { project: next, version, review: draft.review, warnings: [] };
  }

  if (ctx.state === 'demo') {
    const seeded: [string, string, number][] = [
      ['sword-and-shield-idle', 'sword_idle', 1],
      ['sword-and-shield-block', 'sword_block', 1],
      ['standing-dodge-left', 'dodge_left', 1],
    ];
    records.set(keyOf('frostveil', 'elsheria'), {
      map: [],
      follow: [],
      baked: seeded
        .filter(([id]) => ctx.clips().some(c => c.id === id))
        .map(([id, name, revision]) => {
          const clip = clipOf(id);
          return {
            setting: {
              clip: id,
              name,
              start: 0,
              end: clip.frames,
              loops: clip.loops,
              rootMotion: Boolean(clip.rootMotion),
              speed: 1,
            },
            revision,
            libraryName: clip.name,
            version: 2,
            libraryFrames: clip.frames,
          };
        }),
      review: null,
      reviewVersion: null,
    });
  }

  /** Every use of a library clip, for the Animations view. */
  function uses(clip: AnimationClip): ClipUse[] {
    return [...records].flatMap(([key, rec]) => {
      const [projectId = '', assetId = ''] = key.split('/');
      const project = ctx.project(projectId);
      const asset = (project.assets ?? []).find(a => a.id === assetId);
      return rec.baked
        .filter(b => b.setting.clip === clip.id && asset)
        .map(b => ({
          project: project.id,
          projectName: project.name,
          asset: assetId,
          assetName: asset?.name ?? assetId,
          name: b.setting.name,
          version: b.version,
          revision: b.revision,
          outdated: clip.revision > b.revision,
        }));
    });
  }

  /** What the project took from the library, as its `animations.json` says. */
  function ofProject(project: string): ProjectAnimations {
    const assets = [...records]
      .filter(([key]) => key.startsWith(`${project}/`))
      .map(([key, rec]) => [key.slice(project.length + 1), rec] as const);
    return { version: 1, assets: Object.fromEntries(assets) };
  }

  /** The clips a bake put in a model's file, which the bundled file does not hold. */
  function bakedClips(project: string, asset: string, from: number): Clip[] {
    return record(project, asset).baked.map((b, i) =>
      clipInFile(b, clipOf(b.setting.clip), from + i)
    );
  }

  return {
    uses,
    bakedClips,
    handlers: {
      animations_of_project: (args: Args) => ofProject(args.project as string),
      animations_retarget_plan: (args: Args) =>
        plan(
          args.project as string,
          args.asset as string,
          args.clips as string[],
          args.picked as MapChoice[]
        ),
      animations_retarget: (args: Args) => retarget(args.request as UseClips),
      animations_bake: (args: Args) => bake(args.token as string),
      animations_open_draft_in_blender: () =>
        fail('unavailable', 'Mock mode does not open Blender.'),
    } as Record<string, (args: Args) => unknown>,
  };
}
