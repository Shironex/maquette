import type { PackSource } from '@/ipc/client';
import { lastRunLabel, readLastRun } from '@/lib/waiting';
import { renderWait } from '@/state/renders';
import { useRenderStore } from '@/state/use-render-store';
import { useRenderTabStore } from '@/state/use-render-tab-store';

/** The tab's settings and what its buttons do. */
export function useRenderOptions(source: PackSource, assetId: string) {
  const style = useRenderTabStore(s => s.style);
  const settings = useRenderTabStore(s => s.settings);
  const change = useRenderTabStore(s => s.change);
  const reset = useRenderTabStore(s => s.reset);
  const preview = useRenderTabStore(s => s.preview);
  const rendering = useRenderStore(s => s.busy || s.live?.state === 'running');
  const store = useRenderStore.getState;
  return {
    style,
    settings,
    change,
    reset,
    rendering,
    lastRender: lastRunLabel(readLastRun(renderWait(style, settings))),
    dir: preview?.dir ?? null,
    folder: preview?.folder ?? '',
    render: () => void store().renderOne(source, assetId, style, settings),
    addToBatch: () => void store().addToBatch(source, assetId, style, settings),
  };
}
