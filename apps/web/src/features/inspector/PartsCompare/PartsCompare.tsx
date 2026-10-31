import { Suspense } from 'react';
import { StageLoader } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
import { EmptyState, Icon, SegmentedControl, cn } from '@maquette/ui';
import type { Asset, AssetParts } from '@/ipc/client';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { sideLabel } from '@/state/parts-compare';
import { badge } from '../badge';
import type { CompareCanvasHandlers } from '../CompareCanvas';
import { SplitHandle } from '../SplitHandle';
import { KeepRow, NumbersTable } from './NumbersTable';
import { usePartsCompare, type ComparePartsMode } from './PartsCompare.hooks';

const CompareCanvas = lazyAfterPaint(() => import('../CompareCanvas'));

const MODES: readonly { value: ComparePartsMode; label: string }[] = [
  { value: 'shaded', label: 'Shaded' },
  { value: 'parts', label: 'Colour by part' },
  { value: 'wireframe', label: 'Wireframe' },
];

const quiet = () => undefined;

export interface PartsCompareProps {
  asset: Asset;
  found: AssetParts;
  /** The asset's fix-ups changed, so its parts read again. */
  onChanged: () => void;
  handlers?: Partial<CompareCanvasHandlers>;
}

/** Before and after a split, side by side with one camera, and their numbers. */
export default function PartsCompare({ asset, found, onChanged, handlers }: PartsCompareProps) {
  const c = usePartsCompare(asset, found, onChanged);
  if (!c.sides) {
    return (
      <div className="grid flex-1 place-items-center rounded-[14px] border border-line">
        <EmptyState
          icon="columns"
          title="Nothing to compare yet"
          body="Split it into parts, or bring in a second version, to see the two side by side."
        />
      </div>
    );
  }
  const [left, right] = c.sides;
  const [leftUrl, rightUrl] = c.urls;
  const canvasHandlers: CompareCanvasHandlers = {
    onSample: quiet,
    onPick: quiet,
    onStopShots: quiet,
    onUserMove: quiet,
    onLightmaps: quiet,
    onError: quiet,
    onReady: quiet,
    ...handlers,
  };
  const leftLabel = sideLabel(left, c.report?.left ?? null);
  const rightLabel = sideLabel(right, c.report?.right ?? null);
  const keepLabel = (side: typeof left) =>
    side.fixed
      ? 'Keep the split'
      : found.split
        ? `Keep v${side.version} as it came`
        : `Keep v${side.version}`;
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex items-center gap-3">
        <span className="flex-1 text-meta text-fg-mute">
          {asset.name} v{left.version} vs v{right.version}
          {left.version === right.version ? ' after the split' : ''}
        </span>
        {found.game ? (
          <label className="flex items-center gap-2 text-meta text-fg-dim">
            Explode parts
            <input
              type="range"
              min={0}
              max={100}
              value={c.spread}
              aria-label="Explode parts"
              onChange={e => c.setSpread(Number(e.target.value))}
            />
            <span className="w-8 font-mono tabular-nums">{c.spread}%</span>
          </label>
        ) : null}
        <SegmentedControl
          label="Compare view"
          options={MODES}
          value={c.mode}
          onChange={c.setMode}
        />
      </div>
      <div className="relative min-h-[300px] flex-1 overflow-hidden rounded-[14px] border border-line bg-studio">
        {leftUrl && rightUrl ? (
          <CanvasBoundary resetKey={`${leftUrl}|${rightUrl}`}>
            <Suspense fallback={<StageLoader />}>
              <CompareCanvas
                className="absolute! inset-0"
                kind="asset"
                left={[{ id: 'left', url: leftUrl, position: [0, 0, 0] }]}
                right={[{ id: 'right', url: rightUrl, position: [0, 0, 0] }]}
                split={c.split}
                mode={c.mode}
                turntable={false}
                stops={[]}
                stop={null}
                explode={c.spread / 100}
                handlers={canvasHandlers}
              />
            </Suspense>
          </CanvasBoundary>
        ) : null}
        <SplitHandle split={c.split} onChange={c.setSplit} />
        <span className={cn(badge, 'top-3 left-3 font-mono')}>{leftLabel}</span>
        <span className={cn(badge, 'top-3 right-3 font-mono')}>{rightLabel}</span>
        <span className={cn(badge, 'bottom-3 left-3')}>
          <Icon name="camera" size={12} />
          Camera linked
        </span>
        {c.error ? (
          <span className="absolute inset-x-6 top-1/2 -translate-y-1/2 rounded-card bg-danger-soft px-3 py-2 text-center text-danger">
            {c.error}
          </span>
        ) : null}
      </div>
      <NumbersTable
        label="Before and after"
        leftLabel={leftLabel}
        rightLabel={rightLabel}
        rows={c.rows}
      />
      <KeepRow
        busy={c.busy}
        tooltip="The kept one is what the Board and the next export use"
        left={{ label: keepLabel(left), onKeep: () => void c.keep(left) }}
        right={{ label: keepLabel(right), onKeep: () => void c.keep(right) }}
      />
    </div>
  );
}
