/** What a project records of the library clips baked into its models, in mock mode. */
import type {
  AnimationClip,
  Clip,
  ClipSetting,
  FollowChoice,
  MapChoice,
  RetargetReview,
} from '../client';

/** A library clip baked into a model version, as the project records it. */
export interface BakedClip {
  setting: ClipSetting;
  revision: number;
  libraryName: string;
  version: number;
  libraryFrames: number;
}

export interface AssetRecord {
  map: MapChoice[];
  follow: FollowChoice[];
  baked: BakedClip[];
  review: RetargetReview | null;
  reviewVersion: number | null;
}

/** A baked clip as it reads from the model's file, which the bundled fixture does not hold. */
export function clipInFile(baked: BakedClip, library: AnimationClip, index: number): Clip {
  const { setting } = baked;
  const frames = setting.end - setting.start;
  const fps = library.fps ?? 30;
  return {
    name: setting.name,
    index,
    duration: frames / fps,
    keys: frames + 1,
    frames,
    fps,
    channels: 66,
    targets: 22,
    bytes: library.bytes ?? 0,
    shippedBytes: null,
    rootTravel: setting.rootMotion ? (library.rootMotion?.metres ?? 0) : 0,
    loops: setting.loops,
  };
}
