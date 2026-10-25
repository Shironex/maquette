import type { LayoutModel } from '@/ipc/client';
import type { CameraPose, Vec3 } from '@/state/inspector';
import type { PlainLayout, PlainPlacement, Tool } from '@/state/layout';
import type { Selection } from '@/state/use-layout-store';

/** What the canvas tells the editor. */
export interface LayoutCanvasHandlers {
  onSelect: (selection: Selection) => void;
  onBeginDrag: () => void;
  onDragModel: (id: string, placement: PlainPlacement) => void;
  onDragStop: (id: string, position: Vec3) => void;
  /** Where the camera stands after the person moved it, or after a flight. */
  onPose: (pose: CameraPose) => void;
  /** The camera was moved by hand, so it no longer looks through a stop. */
  onUserMove: () => void;
  onError: (message: string) => void;
  onReady: () => void;
}

export interface LayoutCanvasProps {
  models: LayoutModel[];
  layout: PlainLayout;
  selection: Selection;
  tool: Tool;
  snap: boolean;
  grid: boolean;
  preview: { id: string; at: number } | null;
  /** A counter: each change frames every model again. */
  frame: number;
  handlers: LayoutCanvasHandlers;
  className?: string;
}
