import { Suspense, lazy } from 'react';
import {
  Button,
  CheckRow,
  EmptyState,
  Icon,
  IconButton,
  KeyValueList,
  SegmentedControl,
  Select,
  cn,
} from '@maquette/ui';
import { formatAgo } from '@/lib/format';
import { checkSummary, itemTitle, kindHint, reviewPrompt } from '@/state/inbox';
import { useAnimationsStore } from '@/state/use-animations-store';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { CheckFixes } from '../CheckFixes';
import { InboxPrep } from '../InboxPrep';
import { BADGE, KINDS, MODE_ICON } from './constants';
import { useInboxDetail, useInboxPreview } from './InboxDetail.hooks';
import type { InboxDetailProps } from './InboxDetail.types';

const ModelPreview = lazy(() => import('@/features/viewer/ModelPreview'));

/** One inbox model: the 3D preview, its checks and source, and Accept to Blender. */
export default function InboxDetail({ item }: InboxDetailProps) {
  const {
    now,
    project,
    maker,
    checks,
    source,
    isBusy,
    acceptTitle,
    reprocess,
    accept,
    openInBlender,
    setKind,
    redoBlocked,
    redo,
  } = useInboxDetail(item);
  const preview = useInboxPreview(item, project?.id ?? null);
  const { mode, setMode, setLoadError, loadError, url, modes, modeLabel, noPreview, orbitHint } =
    preview;
  const passed = item.worst === 'pass' && item.state !== 'failed';
  const hasClips = Boolean(
    item.report && item.report.rig !== 'none' && item.report.animations.length
  );
  const checkRows = checks.map(c => (
    <CheckRow
      key={c.id}
      status={c.status}
      title={c.title}
      detail={c.detail ?? undefined}
      footer={c.fixes.length ? <CheckFixes item={item} check={c} /> : undefined}
    />
  ));

  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1">
        <section className="flex min-w-0 flex-1 flex-col gap-3.5 px-6 pt-[18px] pb-5">
          <div className="flex items-center gap-3">
            <div className="min-w-0 flex-1">
              <h2 className="truncate text-panel-title">{itemTitle(item)}</h2>
              <p className="mt-0.5 text-fg-mute">
                {maker ? `${maker} · ` : ''}arrived {formatAgo(item.modifiedMs ?? now, now)}
              </p>
            </div>
            <SegmentedControl
              label="Preview mode"
              options={modes}
              value={mode}
              onChange={setMode}
            />
            <IconButton
              icon="arrow-clockwise"
              label={preview.turning ? 'Stop rotating' : 'Rotate the view'}
              size="sm"
              variant="secondary"
              aria-pressed={preview.turning}
              onClick={preview.toggleTurning}
              className={cn(preview.turning && 'border-clay-line text-clay-text')}
            />
          </div>
          <div className="relative min-h-0 flex-1 overflow-hidden rounded-[14px] border border-line bg-studio">
            {url && item.state === 'ready' ? (
              <CanvasBoundary resetKey={item.id}>
                <Suspense fallback={null}>
                  <ModelPreview
                    url={url}
                    mode={mode}
                    turntable={preview.turning}
                    onError={setLoadError}
                    className="absolute inset-0"
                  />
                </Suspense>
              </CanvasBoundary>
            ) : null}
            {noPreview ? (
              <div className="absolute inset-0 grid place-items-center p-6">
                <EmptyState
                  icon="warning-circle"
                  title="No preview for this file"
                  body={loadError ?? item.error ?? 'Reprocess it once the download has finished.'}
                />
              </div>
            ) : null}
            <span className={cn(BADGE, 'top-3 left-3')}>
              <Icon name={MODE_ICON[mode]} size={12} />
              {modeLabel}
            </span>
            {orbitHint ? (
              <span className={cn(BADGE, 'right-3 bottom-3')}>
                <Icon name="hand-pointing" size={12} />
                Drag to orbit · scroll to zoom
              </span>
            ) : null}
          </div>
        </section>
        <aside
          aria-label="Automatic checks"
          className="flex w-[360px] flex-none flex-col gap-3.5 overflow-y-auto border-l border-line bg-panel p-[18px]"
        >
          <div className="flex items-baseline justify-between">
            <h2 className="text-[15px] leading-5 font-semibold">Automatic checks</h2>
            <span className="text-meta text-fg-mute">{checkSummary(item.checks)}</span>
          </div>
          <div className="flex flex-col gap-2">
            {item.state === 'failed' ? (
              <CheckRow
                status="fail"
                title="Could not read the file"
                detail={item.error ?? undefined}
              />
            ) : null}
            {checkRows}
          </div>
          <div className="h-px bg-line" />
          <Select
            label="Kind"
            hint={kindHint(item)}
            options={KINDS}
            value={item.kind}
            onChange={kind => void setKind(item.id, kind)}
          />
          <div className="h-px bg-line" />
          <InboxPrep item={item} project={project} />
          <div className="h-px bg-line" />
          <div className="flex flex-col gap-2">
            <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
              Source
            </h3>
            <KeyValueList items={source} />
          </div>
        </aside>
      </div>
      <div className="flex h-16 flex-none items-center gap-2.5 border-t border-line bg-panel pr-5 pl-6">
        <Icon
          name={passed ? 'check-circle' : 'warning-circle'}
          className={cn('flex-none', passed ? 'text-sage' : 'text-clay-text')}
        />
        <span className="min-w-0 truncate" title={reviewPrompt(item)}>
          {reviewPrompt(item)}
        </span>
        <span className="ml-auto" />
        <IconButton
          icon="arrow-clockwise"
          variant="secondary"
          label="Reprocess"
          hint="Read the file again and rerun the checks"
          disabled={isBusy}
          disabledReason="Working on this model"
          onClick={() => void reprocess(item.id)}
        />
        <Button
          icon="arrow-bend-up-right"
          disabled={Boolean(redoBlocked)}
          disabledReason={redoBlocked ?? undefined}
          tooltip="Send it again with the same settings, changed where you like. This model stays here to compare."
          onClick={redo}
        >
          Redo...
        </Button>
        {hasClips ? (
          <Button
            variant="ghost"
            icon="download-simple"
            tooltip="Keeps this model's clips in the animation library, skeleton only"
            onClick={() =>
              void useAnimationsStore
                .getState()
                .openSave({ project: null, asset: null, inbox: item.id })
            }
          >
            Save clips to library
          </Button>
        ) : null}
        <Button
          variant="ghost"
          icon="arrow-square-out"
          tooltip="Opens this model in Blender to look at or fix it"
          onClick={() => void openInBlender(item.id)}
        >
          Open in Blender
        </Button>
        <Button
          variant="primary"
          icon="check"
          disabled={isBusy || !project || item.state === 'failed'}
          loading={isBusy}
          disabledReason={item.state === 'failed' ? 'This file could not be read' : acceptTitle}
          tooltip={acceptTitle}
          onClick={() => project && void accept(item.id, project.id)}
        >
          Accept to Blender
        </Button>
      </div>
    </div>
  );
}
