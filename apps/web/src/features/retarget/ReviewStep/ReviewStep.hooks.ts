import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { reviewCounts, reviewOrder } from '@/state/retarget';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRetargetStore } from '@/state/use-retarget-store';

/**
 * The review: every baked clip graded, the selected one playing on the
 * clip's skeleton and on the baked copy side by side, on one clock.
 */
export function useReviewStep() {
  const sheet = useRetargetStore(s => s.sheet);
  const change = useRetargetStore(s => s.change);
  const openInBlender = useRetargetStore(s => s.openInBlender);
  const view = useAnimationsStore(s => s.view);
  const items = view?.items ?? [];
  const acceptAsIs = useRetargetStore(s => s.acceptAsIs);
  const [time, setTime] = useState(0);
  const [playing, setPlaying] = useState(true);
  const draft = sheet?.draft ?? null;
  const clips = draft ? reviewOrder(draft.review.clips) : [];
  const selected = clips.find(c => c.clip === sheet?.selected) ?? clips[0] ?? null;
  const source = items.find(i => i.clip.id === selected?.clip) ?? null;
  const fps = source?.clip.fps ?? 30;
  const length = (selected?.frames ?? 0) / fps;
  // The exporter writes a model's clips by name, so the baked clip's index
  // is its place among the model's clips and the new ones, sorted.
  const names = [
    ...new Set([...(sheet?.plan?.modelClips ?? []), ...clips.map(c => c.name)]),
  ].sort();
  return {
    draft,
    clips,
    selected,
    source,
    counts: draft ? reviewCounts(draft.review.clips) : null,
    sourceUrl: source ? fileUrl(source.file) : null,
    draftUrl: draft ? fileUrl(draft.file) : null,
    draftClip: selected ? Math.max(0, names.indexOf(selected.name)) : 0,
    time,
    frame: Math.min(selected?.frames ?? 0, Math.round(time * fps)),
    tick: (delta: number) => {
      if (playing) setTime(t => (length > 0 ? (t + delta) % length : 0));
    },
    playing,
    togglePlay: () => setPlaying(p => !p),
    seek: (frame: number) => {
      setPlaying(false);
      setTime(frame / fps);
    },
    accepted: sheet?.accepted ?? [],
    accept: acceptAsIs,
    openInBlender: () => void openInBlender(),
    select: (clip: string) => {
      setTime(0);
      setPlaying(true);
      change({ selected: clip });
    },
  };
}
