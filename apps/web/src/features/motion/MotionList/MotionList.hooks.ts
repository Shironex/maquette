import { useState } from 'react';
import type { MotionModel } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatBytes } from '@/lib/format';
import { projectFile } from '@/state/board';
import {
  STILL_ROWS,
  clipsToFit,
  matchesFilter,
  modelLine,
  moves,
  n,
  shippedMotion,
  type MotionFilter,
} from '@/state/motion';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';

export interface ListRow {
  model: MotionModel;
  line: string;
  /** What the model weighs on the page with the draft's clips. */
  size: string;
  thumb: string | null;
  over: boolean;
}

/** The models that move and the ones that do not yet, filtered, with the selection. */
export function useMotionList() {
  const workspace = useAppStore(s => s.workspace);
  const view = useMotionStore(s => s.view);
  const draft = useMotionStore(s => s.draft);
  const filter = useMotionStore(s => s.filter);
  const setFilter = useMotionStore(s => s.setFilter);
  const asset = useMotionStore(s => s.asset);
  const select = useMotionStore(s => s.select);
  const [expanded, setExpanded] = useState(false);
  const models = view?.models ?? [];
  // Over marks the models whose clips would have to stay off for the scene to fit.
  const fit = view ? clipsToFit(view, draft) : [];
  const row = (model: MotionModel): ListRow => {
    const bytes = n(model.bytes) - (n(model.motionBytes) - shippedMotion(model, draft));
    const path = model.thumbnail && workspace ? projectFile(workspace, model.thumbnail) : null;
    return {
      model,
      line: modelLine(model, draft),
      size: formatBytes(bytes),
      thumb: fileUrl(path),
      over: fit.some(f => f.model.asset === model.asset),
    };
  };
  const shown = models.filter(m => matchesFilter(m, filter));
  const moving = shown.filter(m => moves(m, draft)).map(row);
  const still = shown.filter(m => !moves(m, draft)).map(row);
  return {
    total: models.length,
    count: models.filter(m => moves(m, draft)).length,
    filter,
    setFilter: (f: MotionFilter) => setFilter(f),
    asset,
    select,
    moving,
    still: expanded ? still : still.slice(0, STILL_ROWS),
    hidden: expanded ? 0 : Math.max(0, still.length - STILL_ROWS),
    stillCount: still.length,
    expand: () => setExpanded(true),
  };
}
