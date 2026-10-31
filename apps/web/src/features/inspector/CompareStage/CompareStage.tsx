import { Suspense } from 'react';
import { StageLoader } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
import { Icon, cn } from '@maquette/ui';
import type { SidePick, Stop } from '@/state/inspector';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { badge } from '../badge';
import type { CompareCanvasHandlers, CompareModel } from '../CompareCanvas';
import { SplitHandle } from '../SplitHandle';
import { useCompareStage } from './CompareStage.hooks';

const CompareCanvas = lazyAfterPaint(() => import('../CompareCanvas'));

export interface CompareStageProps {
  pick: SidePick;
  kind: 'scene' | 'asset';
  left: CompareModel[];
  right: CompareModel[];
  turntable: boolean;
  stops: Stop[];
  /** Lightmapped meshes in the lightmap mode, or null before it reports. */
  lightmaps: number | null;
  ready: boolean;
  loadError: string | null;
  handlers: CompareCanvasHandlers;
  /** The project whose material tweaks the models preview. */
  project: string;
}

/** The compare canvas with its split handle, the sides' labels, the camera and the frame rate. */
export default function CompareStage({
  pick,
  kind,
  left,
  right,
  turntable,
  stops,
  lightmaps,
  ready,
  loadError,
  handlers,
  project,
}: CompareStageProps) {
  const { split, setSplit, mode, stop, fps, activeStop, looks } = useCompareStage(stops, project);
  const models = right.length + left.length;
  return (
    <div className="relative min-h-[340px] flex-1 overflow-hidden rounded-window border border-line bg-[#141311]">
      <CanvasBoundary resetKey={`${pick.left?.id ?? ''}|${pick.right?.id ?? ''}`}>
        <Suspense fallback={<StageLoader />}>
          <CompareCanvas
            className="absolute! inset-0"
            kind={kind}
            left={left}
            right={right}
            split={pick.left ? split : 0}
            mode={mode}
            turntable={turntable}
            stops={stops}
            stop={stop}
            handlers={handlers}
            looks={looks}
          />
        </Suspense>
      </CanvasBoundary>
      {pick.left ? <SplitHandle split={split} onChange={setSplit} /> : null}
      {pick.left ? (
        <span className={cn(badge, 'top-3 left-3')}>
          <span className="font-mono">{pick.left.label}</span>
          {pick.left.note ? <span className="text-fg-dim">{pick.left.note}</span> : null}
        </span>
      ) : null}
      <span className={cn(badge, 'top-3 right-3')}>
        <span className="font-mono">{pick.right.label}</span>
        {pick.right.note ? <span className="text-fg-dim">{pick.right.note}</span> : null}
      </span>
      <span className={cn(badge, 'bottom-3 left-3')}>
        <Icon name="camera" size={12} />
        {activeStop ? `${activeStop.name} · perspective` : 'Free orbit · perspective'}
      </span>
      <span className={cn(badge, 'right-3 bottom-3')}>
        <span
          className={cn(
            'size-[7px] rounded-full',
            fps === 0 ? 'bg-line-strong' : fps >= 55 ? 'bg-sage' : 'bg-amber'
          )}
        />
        {fps === 0 ? 'WebGL 2 · starting' : `WebGL 2 · ${Math.round(fps)} fps`}
      </span>
      {!ready && !loadError ? <StageLoader label={`Loading ${models} models…`} /> : null}
      {loadError ? (
        <span className="absolute inset-x-6 top-1/2 -translate-y-1/2 rounded-card bg-danger-soft px-3 py-2 text-center text-danger">
          {loadError}
        </span>
      ) : null}
      {mode === 'lightmap' && lightmaps === 0 ? (
        <span className={cn(badge, 'top-12 left-1/2 -translate-x-1/2')}>
          <Icon name="sun" size={12} />
          No baked lightmap in these models yet. Run Bake light to make one.
        </span>
      ) : null}
    </div>
  );
}
