import type { PosterCamera, StopId } from './manifest';

export interface WorkbenchProps {
  initialStop: StopId;
  reducedMotion: boolean;
  /** Subscribes to stop changes; returns the unsubscribe. */
  subscribe: (listener: (stop: StopId) => void) => () => void;
  onReady: () => void;
  onFailed: (reason: string) => void;
  /** Poster capture only: hold a poster framing and expose the renderer for stats. */
  capture?: PosterCamera | 'stops';
}

/** The scene inside the canvas takes the workbench's props unchanged. */
export type SceneProps = WorkbenchProps;
