import type { PreviewShot, RenderCamera, RenderShot } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { LOOK_PAPER, STYLE_LABEL, isLook } from '@/state/renders';
import { useRenderTabStore } from '@/state/use-render-tab-store';

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
    camera,
    setCamera: (next: RenderCamera) => change({ camera: next }),
    stageUrl: fileUrl(stage?.file),
    paper: paper ?? LOOK_PAPER[isLook(style) ? style : look],
    frames: (stage?.frames ?? []).map(f => fileUrl(f)),
    tiles,
    taken,
    title,
  };
}
