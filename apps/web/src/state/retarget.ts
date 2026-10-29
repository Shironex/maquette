/**
 * The rules of putting library clips on a model: the four steps, what the
 * chosen clips add, the bone map's counts and the review's. Settings stay
 * per model and are never written back into the library.
 */
import type {
  AnimationClip,
  AnimationItem,
  BonePair,
  ClipReview,
  ClipSetting,
  MotionModel,
  ReviewStatus,
} from '@/ipc/client';

export type RetargetStep = 'choose' | 'map' | 'settings' | 'review';

export const STEPS: readonly { id: RetargetStep; label: string }[] = [
  { id: 'choose', label: 'Choose clips' },
  { id: 'map', label: 'Bone map' },
  { id: 'settings', label: 'Clip settings' },
  { id: 'review', label: 'Review' },
];

export const stepIndex = (step: RetargetStep) => STEPS.findIndex(s => s.id === step);

/** The models a clip can go on: the project's rigged ones. */
export const riggedModels = (models: readonly MotionModel[]) => models.filter(m => m.rig);

export interface ChosenTotals {
  clips: number;
  frames: number;
  bytes: number;
  rootMotion: number;
  loops: number;
  /** Clips per pack, biggest first. */
  packs: [string, number][];
}

export function chosenTotals(items: readonly AnimationItem[], chosen: readonly string[]) {
  const clips = items.map(i => i.clip).filter(c => chosen.includes(c.id));
  const packs = new Map<string, number>();
  for (const c of clips) packs.set(c.pack, (packs.get(c.pack) ?? 0) + 1);
  return {
    clips: clips.length,
    frames: clips.reduce((sum, c) => sum + c.frames, 0),
    bytes: clips.reduce((sum, c) => sum + (c.bytes ?? 0), 0),
    rootMotion: clips.filter(c => c.rootMotion).length,
    loops: clips.filter(c => c.loops).length,
    packs: [...packs].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])),
  } satisfies ChosenTotals;
}

/** Bones the person should look at first: no clip bone or no model bone. */
export const needsLook = (p: BonePair) => !p.source || !p.target;

export function mapCounts(map: readonly BonePair[]) {
  const look = map.filter(needsLook).length;
  return {
    matched: map.length - look,
    look,
    picked: map.filter(p => p.how === 'picked').length,
  };
}

/** The settings, with one clip's changed. */
export function withSetting(
  settings: readonly ClipSetting[],
  clip: string,
  change: Partial<ClipSetting>
): ClipSetting[] {
  return settings.map(s => (s.clip === clip ? { ...s, ...change } : s));
}

/**
 * "30 f, plays in 0.87 s at 1.15x". A clip of `frames` frames runs from frame 0 to frame
 * `frames`, so a trim keeps `end - start` of them.
 */
export function playLine(setting: ClipSetting, clip: Pick<AnimationClip, 'fps'>): string {
  const frames = setting.end - setting.start;
  const seconds = frames / (clip.fps || 30) / (setting.speed || 1);
  return `${frames} f, plays in ${seconds.toFixed(2)} s at ${setting.speed}x`;
}

export const trimmed = (setting: ClipSetting, clip: Pick<AnimationClip, 'frames'>) =>
  setting.start > 0 || setting.end < clip.frames;

export function reviewCounts(clips: readonly ClipReview[]): Record<ReviewStatus, number> {
  const count = (s: ReviewStatus) => clips.filter(c => c.status === s).length;
  return { good: count('good'), look: count('look'), failed: count('failed') };
}

/** Looks first, then failures, then the good ones, as the review lists them. */
export function reviewOrder(clips: readonly ClipReview[]): ClipReview[] {
  const rank: Record<ReviewStatus, number> = { look: 0, failed: 1, good: 2 };
  return [...clips].sort((a, b) => rank[a.status] - rank[b.status]);
}

/** A clip Pin the feet can help: flagged for a sliding foot and not pinned yet. */
export const canPin = (c: ClipReview) =>
  c.status === 'look' && c.slideFrames !== null && c.unpinnedCm === null;

/** "4.2 cm", a slide as the review shows it. */
export const slideText = (cm: number) => `${cm.toFixed(1)} cm`;
