import type { LayoutModel, ScenePath } from '@/ipc/client';
import type { Vec3 } from '@/state/inspector';
import type { PlainLayout } from '@/state/layout';
import type { PathSelection, PathTool } from '@/state/paths';

/** What the canvas tells the editor. */
export interface PathsCanvasHandlers {
  onSelect: (selection: PathSelection) => void;
  /** Add points: where a click landed on a model or the ground. */
  onAddPoint: (point: Vec3) => void;
  /** Draw on surfaces: where a drag went over the models, in order. */
  onStroke: (stroke: Vec3[]) => void;
  /** A point moves under the gizmo. */
  onDragPoint: (index: number, point: Vec3) => void;
  /** The gizmo let go of a point. */
  onDropPoint: (index: number, point: Vec3) => void;
  onError: (message: string) => void;
  onReady: () => void;
}

export interface PathsCanvasProps {
  models: LayoutModel[];
  /** Where the models stand. */
  layout: PlainLayout;
  paths: ScenePath[];
  selection: PathSelection;
  tool: PathTool;
  snap: boolean;
  /** A counter: each change frames the scene again. */
  frame: number;
  handlers: PathsCanvasHandlers;
  className?: string;
}
