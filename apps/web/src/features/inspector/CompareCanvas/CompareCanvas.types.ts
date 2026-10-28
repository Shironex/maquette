import type { ModelMaterials } from '@/ipc/client';
import type { Stop } from '@/state/inspector';
import type { InspectMode } from '@/state/use-inspector-store';

export interface CompareModel {
  /** The InspectModel id, reported back when it is clicked. */
  id: string;
  url: string;
  position: [number, number, number];
  /** The asset whose material tweaks the model previews, if any. */
  asset?: string | null;
}

/** What the canvas tells the Inspector. */
export interface CompareCanvasHandlers {
  onSample: (ms: number, fps: number) => void;
  onPick: (model: string | null) => void;
  /** Live renders of the stops without a poster, by stop index. */
  onStopShots: (shots: Record<number, string>) => void;
  /** The camera was moved by hand. */
  onUserMove: () => void;
  onLightmaps: (meshes: number) => void;
  onError: (message: string) => void;
  onReady: () => void;
}

export interface CompareCanvasProps {
  kind: 'scene' | 'asset';
  left: CompareModel[];
  right: CompareModel[];
  /** Where the split sits, from 0 (all right side) to 1 (all left side). */
  split: number;
  mode: InspectMode;
  turntable: boolean;
  stops: Stop[];
  stop: { index: number; at: number } | null;
  handlers: CompareCanvasHandlers;
  /** How far each side's parts pull apart from its middle, 0 to 1. */
  explode?: number;
  /** Material tweaks to preview, by asset id; shown in the shaded mode. */
  looks?: Readonly<Record<string, ModelMaterials>>;
  className?: string;
}
