import { useState } from 'react';
import { formatBytes } from '@/lib/format';
import { budgetLine, clipBytes, clipsToFit, isOff, n } from '@/state/motion';
import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useMotionStore } from '@/state/use-motion-store';

/** The selected model's clips with their ship choices, and whether the scene is over. */
export function useClipsPanel() {
  const view = useMotionStore(s => s.view);
  const draft = useMotionStore(s => s.draft);
  const asset = useMotionStore(s => s.asset);
  const selected = useMotionStore(s => s.clip);
  const selectClip = useMotionStore(s => s.selectClip);
  const setShip = useMotionStore(s => s.setShip);
  const setBoneMap = useMotionStore(s => s.setBoneMap);
  const setView = useAppStore(s => s.setView);
  const setAdding = useBuiltStore(s => s.setAdding);
  const [menu, setMenu] = useState(false);
  const model = view?.models.find(m => m.asset === asset) ?? null;
  const line = view ? budgetLine(view, draft) : null;
  const fit = view ? clipsToFit(view, draft) : [];
  const rows = (model?.clips ?? []).map(c => ({
    clip: c,
    ship: !isOff(draft, model!.asset, c.name),
    meta: `${c.frames} f · ${n(c.fps)} fps`,
    size: formatBytes(clipBytes(c)),
    over: fit.some(f => f.model.asset === model!.asset && f.clip.name === c.name),
  }));
  return {
    model,
    rows,
    selected,
    selectClip,
    over: Boolean(line?.over),
    supported: view?.clipsOffSupported ?? false,
    setShip: (clip: string, ship: boolean) => model && setShip(model.asset, clip, ship),
    menu,
    setMenu,
    actions: {
      retarget: () => setBoneMap(true),
      inbox: () => setView('inbox'),
      script: () => {
        setView('board');
        setAdding(true);
      },
      pipeline: () => setView('pipeline'),
    },
  };
}
