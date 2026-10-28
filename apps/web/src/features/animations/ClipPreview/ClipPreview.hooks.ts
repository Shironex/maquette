import { useState } from 'react';
import type { AnimationItem } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { travelLine } from '@/state/animations';

/** The clip's own player: the time it loops through, the frame on show and a load failure. */
export function useClipPreview(item: AnimationItem) {
  const { clip } = item;
  const [time, setTime] = useState(0);
  const [failed, setFailed] = useState<string | null>(null);
  const length = clip.length ?? 0;
  const fps = clip.fps ?? 30;
  return {
    url: fileUrl(item.file),
    time,
    frame: Math.min(clip.frames, Math.round(time * fps)),
    travel: travelLine(clip),
    failed,
    setFailed,
    tick: (delta: number) => setTime(t => (length > 0 ? (t + delta) % length : 0)),
  };
}
