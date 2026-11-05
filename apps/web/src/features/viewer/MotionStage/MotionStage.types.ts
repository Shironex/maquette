import type { PartOffset } from '../motion-pose';

export interface MotionStageProps {
  /** A URL the web view can fetch: the asset protocol, or a mock file. */
  url: string | null;
  /**
   * The clip to play, by its place among the file's animations or by its
   * name; none (or a name the file lacks) holds the rest pose.
   */
  clip: number | string | null;
  /** Seconds into the clip, or how far into it as a share of its length. */
  time: number | { share: number };
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
  /** The model is up and framed, or could not be read. */
  report?: { onLoaded?: () => void; onError?: (message: string) => void };
  className?: string;
  label: string;
}
