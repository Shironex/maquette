import { Suspense, lazy } from 'react';
import { SegmentedControl, Select, cn, type SegmentedOption } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import type { GizmoMode } from '@/features/viewer/AssembleStage';
import type { DraftPart } from '@/state/split';
import { useAssembleViewer } from './AssembleViewer.hooks';

const AssembleStage = lazy(() => import('@/features/viewer/AssembleStage'));

const GIZMOS: SegmentedOption<GizmoMode>[] = [
  { value: 'translate', label: 'Move', icon: 'arrows-out-cardinal' },
  { value: 'rotate', label: 'Turn', icon: 'arrow-clockwise', hint: 'Turns it about its pivot' },
];

export interface AssembleViewerProps {
  parts: readonly DraftPart[];
  onPlace: (index: number, patch: Partial<DraftPart>) => void;
}

/** The parts in 3D, with a gizmo that moves or turns the chosen one. */
export default function AssembleViewer({ parts, onPlace }: AssembleViewerProps) {
  const v = useAssembleViewer(parts, onPlace);
  const error = v.error;
  const partOptions = parts.map(p => ({ value: p.asset, label: p.name.trim() || p.asset }));
  return (
    <div className="flex flex-col gap-2">
      <div className="relative h-[260px] overflow-hidden rounded-card border border-line bg-studio">
        <CanvasBoundary resetKey={v.files}>
          <Suspense fallback={null}>
            <AssembleStage
              className="absolute! inset-0"
              parts={v.stage}
              selected={v.selected}
              mode={v.mode}
              snap={v.snap}
              onSelect={v.select}
              onPlace={v.place}
              onError={v.setError}
            />
          </Suspense>
        </CanvasBoundary>
      </div>
      <div className="flex items-center gap-2">
        <Select
          aria-label="Part the gizmo moves"
          size="sm"
          options={partOptions}
          value={v.selected ?? ''}
          onChange={v.select}
          fieldClassName="min-w-0 flex-1"
        />
        <SegmentedControl label="Gizmo" options={GIZMOS} value={v.mode} onChange={v.setMode} />
        <Select
          aria-label={v.mode === 'translate' ? 'Move snapping' : 'Turn snapping'}
          size="sm"
          options={v.snapOptions}
          value={v.snapValue}
          onChange={v.setSnap}
        />
      </div>
      <p className={cn('text-meta', error ? 'text-amber' : 'text-fg-mute')}>
        {error ?? 'Click a part or pick it, then drag the gizmo. The numbers follow each drag.'}
      </p>
    </div>
  );
}
