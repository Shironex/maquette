import type { PreviewShot, RenderCamera, RenderShot } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { stopsOf } from '@/state/inspector';
import { LOOK_PAPER, STYLE_LABEL, aimOf, isLook } from '@/state/renders';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useRenderTabStore } from '@/state/use-render-tab-store';

const CAMERAS: readonly { value: RenderCamera | 'stop'; label: string }[] = [
  { value: 'front', label: 'Front' },
  { value: 'three-quarter', label: 'Three quarter' },
  { value: 'side', label: 'Side' },
  { value: 'stop', label: 'Camera stop' },
];

export interface StripTile {
  key: string;
  label: string;
  url: string | null;
  active: boolean;
  onClick: () => void;
}

const TILE_LABEL = (s: PreviewShot) =>
  s.turn === 0 && isLook(s.style) ? STYLE_LABEL[s.style].toLowerCase() : `${s.turn}°`;

/** The picture on the stage, the strip under it and what the tags say. */
export function useRenderStage() {
  const style = useRenderTabStore(s => s.style);
  const setStyle = useRenderTabStore(s => s.setStyle);
  const preview = useRenderTabStore(s => s.preview);
  const loading = useRenderTabStore(s => s.loading);
  const error = useRenderTabStore(s => s.error);
  const camera = useRenderTabStore(s => s.settings.camera);
  const aim = useRenderTabStore(s => s.settings.stop);
  const view = useInspectorStore(s => s.view);
  const look = useRenderTabStore(s => s.settings.look);
  const paper = useRenderTabStore(s => s.settings.paperColor);
  const change = useRenderTabStore(s => s.change);
  const [stage, ...strip] = preview?.shots ?? [];
  const shot: RenderShot | undefined = stage;
  const tiles: StripTile[] = strip.map(s => ({
    key: `${s.style}-${s.turn}`,
    label: TILE_LABEL(s),
    url: fileUrl(s.file),
    active: s.turn === 0 && s.style === style,
    onClick: () => {
      if (s.turn === 0) setStyle(s.style);
    },
  }));
  const stops = view ? stopsOf(view) : [];
  const stopOptions = stops.map(s => ({ value: s.name, label: s.name }));
  const pickStop = (name: string) => {
    const found = stops.find(s => s.name === name);
    if (found) change({ stop: aimOf(found) });
  };
  const seconds = stage?.seconds;
  const taken =
    shot && seconds != null
      ? `Preview · ${shot.size} px · rendered in ${seconds.toFixed(1)} s`
      : null;
  const height = preview?.heightM != null ? `${preview.heightM.toFixed(2)} m · ` : '';
  const title = preview ? `${preview.name.toUpperCase()} · ${height}v${preview.version}` : null;
  return {
    style,
    preview,
    loading,
    error,
    camera: aim ? ('stop' as const) : camera,
    stopName: aim?.name ?? null,
    stopOptions,
    pickStop,
    cameras: CAMERAS.map(o =>
      o.value === 'stop' && stops.length === 0
        ? { ...o, disabled: true, hint: 'This project has no camera stops yet' }
        : o
    ),
    setCamera: (next: RenderCamera | 'stop') => {
      if (next !== 'stop') return change({ camera: next, stop: null });
      const first = stops[0];
      if (first) change({ stop: aimOf(first) });
    },
    stageUrl: fileUrl(stage?.file),
    paper: paper ?? LOOK_PAPER[isLook(style) ? style : look],
    frames: (stage?.frames ?? []).map(f => fileUrl(f)),
    tiles,
    taken,
    title,
  };
}
