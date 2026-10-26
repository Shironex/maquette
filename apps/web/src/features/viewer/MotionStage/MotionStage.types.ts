import type { PartOffset } from '../motion-pose';

export interface MotionStageProps {
  /** A URL the web view can fetch: the asset protocol, or a mock file. */
  url: string | null;
  /** The clip to play, by its place among the file's animations; none holds the rest pose. */
  clip: number | null;
  /** Seconds into the clip. */
  time: number;
  skeleton: boolean;
  /** The path the root walks over the clip, dashed on the floor. */
  trail: boolean;
  /** Spin slowly until someone drags. */
  turntable: boolean;
  /** Object state offsets by part name, from each part's rest pose. */
  poses: Record<string, PartOffset> | null;
  /** Parts to outline. */
  outline: readonly string[];
  /** Each frame's wall time, for the player to move on. */
  onFrame?: (delta: number) => void;
  onError?: (message: string) => void;
  className?: string;
  label: string;
}
