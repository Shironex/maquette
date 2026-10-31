import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { frameAt, n } from '@/state/motion';
import { useAppStore } from '@/state/use-app-store';
import { bakedClip, useBakedOf } from '@/state/use-baked-store';
import { useInspectorStore } from '@/state/use-inspector-store';
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
  const entry = useBakedOf(asset);
  const from = bakedClip(entry, clip?.name);
  const projectId = useAppStore(s => s.workspace?.project?.id ?? null);
  const setView = useAppStore(s => s.setView);
  const focusVersion = useInspectorStore(s => s.focusVersion);
  const bakedNow = (entry.baked ?? []).filter(b => b.version === model?.version).length;
  const bake =
    model && entry.reviewVersion === model.version && bakedNow > 0 && model.version > 1
      ? {
          title: `v${model.version} is v${model.version - 1} with ${bakedNow} library ${bakedNow === 1 ? 'clip' : 'clips'} baked in`,
          detail: `Made headless in Blender with the bone map you checked; v${model.version - 1} is kept.`,
          compareLabel: `Compare with v${model.version - 1}`,
          compare: () => {
            if (!projectId) return;
            focusVersion(projectId, model.asset, model.version);
            setView('inspector');
          },
        }
      : null;
  const travel = clip?.rootTravel ?? null;
  const badges = model
    ? clip
      ? [
          { icon: 'film-strip' as const, text: `${clip.name} · frame ${frameAt(clip, time)}` },
          ...(travel
            ? [{ icon: 'path' as const, text: `Root motion, ${n(travel).toFixed(2)} m per loop` }]
            : []),
          ...(from ? [{ icon: 'books' as const, text: `from ${from.libraryName}` }] : []),
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
    bake,
    url,
    label,
    badges,
    overlays: { skeleton, setSkeleton, trail, setTrail, turntable, setTurntable },
    failed: (failed?.url === url ? failed.message : null) ?? model?.error ?? null,
    setFailed: (message: string) => setFailed({ url, message }),
  };
}
