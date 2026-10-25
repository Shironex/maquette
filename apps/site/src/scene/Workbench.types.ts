import type { PosterCamera, StopId } from './manifest';
import type { TourScroll } from './tour';

export interface WorkbenchProps {
  /** The stop the page opened on: its models load first. */
  initialStop: StopId;
  reducedMotion: boolean;
  /** The page's tour progress. */
  scroll: TourScroll;
  onReady: () => void;
  onFailed: (reason: string) => void;
  /** Poster capture only: hold a poster framing and expose the renderer for stats. */
  capture?: PosterCamera | 'stops';
}

/** The scene inside the canvas takes the workbench's props unchanged. */
export type SceneProps = WorkbenchProps;
