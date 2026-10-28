import { useEffect, useState } from 'react';
import type { Clip } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { budgetLine, clipBytes, clipsToFit, isOff, n } from '@/state/motion';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useAppStore } from '@/state/use-app-store';
import { useIsGame } from '@/state/use-budget-check';
import { useBuiltStore } from '@/state/use-built-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRetargetStore } from '@/state/use-retarget-store';

/** A clip list long enough to need the search box. */
export const SEARCH_FROM = 8;

/** The clips whose name has every word of `query` in it, ignoring case and `_`. */
export function matchClips(clips: readonly Clip[], query: string): Clip[] {
  const words = query
    .toLowerCase()
    .split(/[\s_]+/)
    .filter(Boolean);
  if (!words.length) return [...clips];
  return clips.filter(c => {
    const name = c.name.toLowerCase().replaceAll('_', ' ');
    return words.every(w => name.includes(w));
  });
}

/**
 * The selected model's clips with their ship choices, and whether the scene
 * is over. A game weighs no clip against a scene budget: Game export keeps
 * every clip in the model, so there are no ship choices either.
 */
export function useClipsPanel() {
  const view = useMotionStore(s => s.view);
  const draft = useMotionStore(s => s.draft);
  const asset = useMotionStore(s => s.asset);
  const selected = useMotionStore(s => s.clip);
  const selectClip = useMotionStore(s => s.selectClip);
  const setShip = useMotionStore(s => s.setShip);
  const openRetarget = useRetargetStore(s => s.open);
  const setView = useAppStore(s => s.setView);
  const projectId = useAppStore(s => s.workspace?.project?.id ?? null);
  const setAdding = useBuiltStore(s => s.setAdding);
  const [menu, setMenu] = useState(false);
  const [query, setQuery] = useState('');
  const game = useIsGame();
  // Another model's clips start unfiltered.
  useEffect(() => setQuery(''), [asset]);
  const model = view?.models.find(m => m.asset === asset) ?? null;
  const line = view ? budgetLine(view, draft) : null;
  const fit = view && !game ? clipsToFit(view, draft) : [];
  const clips = model?.clips ?? [];
  const rows = matchClips(clips, query).map(c => ({
    clip: c,
    ship: !isOff(draft, model!.asset, c.name),
    meta: `${c.frames} f · ${n(c.fps)} fps`,
    size: formatBytes(clipBytes(c)),
    over: fit.some(f => f.model.asset === model!.asset && f.clip.name === c.name),
  }));
  return {
    model,
    rows,
    total: clips.length,
    query,
    setQuery,
    game,
    selected,
    selectClip,
    over: !game && Boolean(line?.over),
    supported: view?.clipsOffSupported ?? false,
    /** A game ships every clip in the model file; say so in place of the switches. */
    gameNote: game && clips.length > 0,
    /** A website whose Build scene script cannot leave clips out yet. */
    oldStarters: !game && clips.length > 0 && !view?.clipsOffSupported,
    setShip: (clip: string, ship: boolean) => model && setShip(model.asset, clip, ship),
    menu,
    setMenu,
    actions: {
      retarget: () => {
        if (model && projectId) void openRetarget({ project: projectId, asset: model.asset });
      },
      save: () => {
        if (model && projectId) {
          void useAnimationsStore
            .getState()
            .openSave({ project: projectId, asset: model.asset, inbox: null });
        }
      },
      inbox: () => setView('inbox'),
      script: () => {
        setView('board');
        setAdding(true);
      },
      pipeline: () => setView('pipeline'),
    },
  };
}
