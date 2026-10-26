import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { frameAt, n } from '@/state/motion';
import { currentClip, useMotionStore } from '@/state/use-motion-store';

/** The selected model and clip, the playhead, and which overlays the viewport shows. */
export function useClipStage() {
  const view = useMotionStore(s => s.view);
  const asset = useMotionStore(s => s.asset);
  const clipName = useMotionStore(s => s.clip);
  const selectClip = useMotionStore(s => s.selectClip);
  const time = useMotionStore(s => s.player.time);
  const tick = useMotionStore(s => s.tick);
  const [skeleton, setSkeleton] = useState(true);
  const [trail, setTrail] = useState(true);
  const [turntable, setTurntable] = useState(false);
  const [failed, setFailed] = useState<{ url: string | null; message: string } | null>(null);
  const model = view?.models.find(m => m.asset === asset) ?? null;
  const url = model ? fileUrl(model.file) : null;
  const clip = currentClip({ view, asset, clip: clipName });
  const travel = clip?.rootTravel ?? null;
  const badges = model
    ? clip
      ? [
          { icon: 'film-strip' as const, text: `${clip.name} · frame ${frameAt(clip, time)}` },
          ...(travel
            ? [{ icon: 'path' as const, text: `Root motion, ${n(travel).toFixed(2)} m per loop` }]
            : []),
        ]
      : [
          { icon: 'person-simple' as const, text: 'Rest pose' },
          ...(model.rig ? [{ icon: 'bone' as const, text: `${model.rig.joints} bones` }] : []),
        ]
    : [];
  const label = model
    ? clip
      ? `${model.name} playing ${clip.name} at frame ${frameAt(clip, time)}`
      : `${model.name} at rest`
    : 'No model';
  return {
    model,
    clip,
    time,
    tick,
    selectClip,
    url,
    label,
    badges,
    overlays: { skeleton, setSkeleton, trail, setTrail, turntable, setTurntable },
    failed: (failed?.url === url ? failed.message : null) ?? model?.error ?? null,
    setFailed: (message: string) => setFailed({ url, message }),
  };
}
