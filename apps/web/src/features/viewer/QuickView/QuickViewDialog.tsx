import { Suspense, lazy } from 'react';
import { Button, EmptyState, Icon, IconButton, SegmentedControl, cn } from '@maquette/ui';
import { KEYS } from '@/lib/keys';
import { quickViewLine } from '@/state/quick-view';
import { CanvasBoundary } from '../CanvasBoundary';
import { BADGE, MODE_ICON } from '../previewModes';
import { useQuickViewDialog } from './QuickView.hooks';
import type { QuickViewDialogProps } from './QuickView.types';

// three.js loads with the first quick view, not with the Board or the Library.
const ModelPreview = lazy(() => import('../ModelPreview'));

/**
 * One model in the same 3D preview as the Inbox, over the screen: orbit and
 * zoom, its preview modes, its numbers and the way into the Inspector.
 * Closing it unmounts the preview, which frees what it put on the GPU.
 */
export default function QuickViewDialog(props: QuickViewDialogProps) {
  const { target, onClose } = props;
  const q = useQuickViewDialog(props);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={q.ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="quick-view-title"
        aria-describedby="quick-view-meta"
        className="flex h-[min(720px,100%)] w-[min(1040px,100%)] flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <div className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="cube" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="quick-view-title" className="truncate text-panel-title">
              {target.name}
            </h2>
            <p id="quick-view-meta" className="truncate text-meta text-fg-mute">
              {quickViewLine(target)}
            </p>
          </div>
          <SegmentedControl
            label="Preview mode"
            options={q.modes}
            value={q.mode}
            onChange={q.setMode}
          />
          <IconButton
            icon="arrow-clockwise"
            label={q.turning ? 'Stop rotating' : 'Rotate the view'}
            variant="secondary"
            aria-pressed={q.turning}
            onClick={q.toggleTurning}
            className={cn(q.turning && 'border-clay-line text-clay-text')}
          />
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={onClose} />
        </div>
        <div className="relative m-4 min-h-0 flex-1 overflow-hidden rounded-[14px] border border-line bg-studio">
          {q.url ? (
            <CanvasBoundary resetKey={target.file}>
              <Suspense fallback={null}>
                <ModelPreview
                  url={q.url}
                  mode={q.mode}
                  turntable={q.turning}
                  onLoaded={q.onLoaded}
                  onError={q.setLoadError}
                  className="absolute inset-0"
                />
              </Suspense>
            </CanvasBoundary>
          ) : null}
          {q.showing ? null : (
            <div className="absolute inset-0 grid place-items-center p-6">
              <EmptyState
                icon="warning-circle"
                title="No preview for this file"
                body={q.loadError ?? 'The model file is not where the project says it is.'}
              />
            </div>
          )}
          <span className={cn(BADGE, 'top-3 left-3')}>
            <Icon name={MODE_ICON[q.mode]} size={12} />
            {q.modeLabel}
          </span>
          {q.showing ? (
            <span className={cn(BADGE, 'right-3 bottom-3')}>
              <Icon name="hand-pointing" size={12} />
              Drag to orbit · scroll to zoom
            </span>
          ) : null}
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          {q.inspect || !target.inspectBlocked ? null : (
            <p className="mr-auto min-w-0 truncate text-meta text-fg-mute">
              {target.inspectBlocked}.
            </p>
          )}
          <Button
            variant="primary"
            icon="magnifying-glass"
            disabled={!q.inspect}
            disabledReason={target.inspectBlocked ?? undefined}
            tooltip="Opens this version in the Inspector to compare, measure and tweak it"
            onClick={() => q.inspect?.()}
          >
            Open in Inspector
          </Button>
        </div>
      </div>
    </div>
  );
}
