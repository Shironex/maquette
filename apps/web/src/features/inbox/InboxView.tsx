import { Suspense, lazy, useState } from 'react';
import {
  Button,
  CheckRow,
  EmptyState,
  Icon,
  KeyValueList,
  SegmentedControl,
  Select,
  StatusChip,
  TopBar,
  cn,
  type IconName,
} from '@maquette/ui';
import type { AssetKind, InboxItem, ProjectManifest } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatAgo, formatBytes } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import {
  acceptTarget,
  checkSummary,
  contentKey,
  firstToReview,
  itemChip,
  itemMeta,
  itemTitle,
  madeBy,
  reviewPrompt,
  sentFor,
  sortChecks,
  kindHint,
} from '@/state/inbox';
import { PROVIDER_LABEL } from '@/state/providers';
import { KIND_ICON } from '@/state/thumbs';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import type { PreviewMode } from '@/features/viewer/ModelPreview';

const NO_PROJECTS: readonly ProjectManifest[] = [];

const ModelPreview = lazy(() => import('@/features/viewer/ModelPreview'));

const MODES: readonly { value: PreviewMode; label: string }[] = [
  { value: 'shaded', label: 'Shaded' },
  { value: 'wireframe', label: 'Wireframe' },
  { value: 'uv', label: 'UV' },
  { value: 'bones', label: 'Bones' },
];

const MODE_ICON: Record<PreviewMode, IconName> = {
  shaded: 'cube',
  wireframe: 'cube-transparent',
  uv: 'squares-four',
  bones: 'person-simple',
};

/** A label laid over the viewport, as the canvas draws them. */
const BADGE =
  'pointer-events-none absolute flex h-[22px] items-center gap-[5px] rounded-[6px] bg-[rgba(28,27,25,0.78)] px-2 text-[11.5px] font-medium text-fg';

const KINDS: readonly { value: AssetKind; label: string }[] = [
  { value: 'prop', label: 'Prop' },
  { value: 'character', label: 'Character' },
  { value: 'scene', label: 'Scene' },
];

export function InboxView() {
  const { snapshot, selected, select } = useInboxStore();
  const items = snapshot?.items ?? [];
  const item = items.find(i => i.id === selected) ?? firstToReview(items);

  return (
    <>
      <TopBar lead={<h1 className="text-panel-title">Inbox</h1>} />
      <main className="flex min-h-0 flex-1">
        <section
          aria-label="Models in the inbox"
          className="flex w-[300px] flex-none flex-col border-r border-line bg-panel"
        >
          <div className="px-4 pt-[18px] pb-3">
            <h2 className="text-panel-title">Model inbox</h2>
            <p className="mt-0.5 text-fg-mute">
              {items.length === 0
                ? 'Empty'
                : `${items.length} ${items.length === 1 ? 'model' : 'models'} waiting`}
            </p>
          </div>
          <ul className="flex min-h-0 flex-1 list-none flex-col gap-1 overflow-y-auto px-2">
            {items.map(i => (
              <li key={i.id}>
                <InboxRow item={i} selected={i.id === item?.id} onSelect={() => select(i.id)} />
              </li>
            ))}
          </ul>
          <div className="flex items-center gap-2 border-t border-line px-4 py-3">
            <span
              className={cn('size-2 rounded-full', snapshot?.watching ? 'bg-sage' : 'bg-fg-mute')}
            />
            <span className="flex-1 truncate font-mono text-[12px] text-fg-mute">
              {snapshot?.dir ?? 'inbox'}
            </span>
            <span className="text-meta text-fg-mute">
              {snapshot?.watching ? 'watching' : 'not watched'}
            </span>
          </div>
        </section>
        {/* A finished download or a new file under the same name starts the preview over. */}
        {item ? (
          <InboxDetail key={contentKey(item)} item={item} />
        ) : (
          <InboxEmpty dir={snapshot?.dir} />
        )}
      </main>
    </>
  );
}

function InboxRow({
  item,
  selected,
  onSelect,
}: {
  item: InboxItem;
  selected: boolean;
  onSelect: () => void;
}) {
  const chip = itemChip(item);
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className={cn(
        'flex w-full cursor-pointer items-center gap-3 rounded-card p-2 text-left',
        selected ? 'bg-clay-soft shadow-[inset_0_0_0_1px_var(--clay-line)]' : 'hover:bg-hover'
      )}
    >
      <RowThumb item={item} />
      <span className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-semibold">{itemTitle(item)}</span>
        <span className="truncate font-mono text-[11.5px] leading-[14px] text-fg-mute">
          {itemMeta(item)}
        </span>
        <span>
          <StatusChip status={chip.status} label={chip.label} />
        </span>
      </span>
    </button>
  );
}

/** The studio render, or the kind icon while it renders (or for a file that cannot be read). */
function RowThumb({ item }: { item: InboxItem }) {
  const src = fileUrl(item.thumbnail);
  if (src) {
    return (
      <img
        src={src}
        alt=""
        draggable={false}
        className="size-[52px] flex-none rounded-[8px] bg-studio object-cover"
      />
    );
  }
  const rendering = item.state === 'ready' && item.thumbnailState === 'rendering';
  return (
    <span
      title={rendering ? 'Rendering a thumbnail' : undefined}
      className="grid size-[52px] flex-none place-items-center rounded-[8px] border border-line bg-raised text-fg-mute"
    >
      <Icon
        name={item.state === 'failed' ? 'warning-circle' : KIND_ICON[item.kind]}
        size={20}
        className={cn(rendering && 'animate-pulse motion-reduce:animate-none')}
      />
    </span>
  );
}

function InboxEmpty({ dir }: { dir?: string }) {
  return (
    <div className="grid flex-1 place-items-center p-6">
      <EmptyState
        icon="tray"
        title="Nothing in the inbox"
        body={`Drop a .glb from Tripo into ${dir ?? 'the inbox folder'}. I check each model before it reaches Blender.`}
      />
    </div>
  );
}

function InboxDetail({ item }: { item: InboxItem }) {
  const [mode, setMode] = useState<PreviewMode>('shaded');
  const [loadError, setLoadError] = useState<string | null>(null);
  const now = useNow(30_000);
  const current = useAppStore(s => s.workspace?.project ?? null);
  const projects = useAppStore(s => s.workspace?.projects) ?? NO_PROJECTS;
  const project = acceptTarget(item, projects, current);
  const sentLabel = item.link
    ? sentFor(item, projects, current?.id ?? null, PROVIDER_LABEL[item.link.provider])
    : null;
  const { busy, reprocess, accept, openInBlender, setKind } = useInboxStore();
  const report = item.report;
  const maker = madeBy(item);
  const checks = sortChecks(item.checks);
  const url = fileUrl(item.file);
  const isBusy = busy === item.id;
  const hasBones = (report?.joints ?? 0) > 0;
  const source = [
    {
      label: 'Kind',
      value: item.kind === 'character' ? 'Character' : item.kind === 'scene' ? 'Scene' : 'Prop',
    },
    { label: 'Size', value: formatBytes(item.bytes ?? 0), mono: true },
    ...(sentLabel ? [{ label: 'Sent for', value: sentLabel }] : []),
    ...(report
      ? [
          {
            label: 'Meshes',
            value: `${report.meshes} · ${report.primitives} primitives`,
            mono: true,
          },
          { label: 'Materials', value: String(report.materials), mono: true },
          ...(maker ? [{ label: 'Made by', value: maker }] : []),
        ]
      : []),
    { label: 'File', value: item.id, mono: true },
  ];

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
              options={MODES.filter(m => m.value !== 'bones' || hasBones)}
              value={mode}
              onChange={setMode}
            />
          </div>
          <div className="relative min-h-0 flex-1 overflow-hidden rounded-[14px] border border-line bg-studio">
            {url && item.state === 'ready' ? (
              <CanvasBoundary resetKey={item.id}>
                <Suspense fallback={null}>
                  <ModelPreview
                    url={url}
                    mode={mode}
                    onError={setLoadError}
                    className="absolute inset-0"
                  />
                </Suspense>
              </CanvasBoundary>
            ) : null}
            {!url || loadError || item.state === 'failed' ? (
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
              {MODES.find(m => m.value === mode)?.label}
            </span>
            {url && item.state === 'ready' && !loadError ? (
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
            {checks.map(c => (
              <CheckRow
                key={c.id}
                status={c.status}
                title={c.title}
                detail={c.detail ?? undefined}
              />
            ))}
          </div>
          <div className="h-px bg-line" />
          <Select
            label="Kind"
            hint={kindHint(item)}
            options={KINDS}
            value={item.kind}
            onChange={kind => void setKind(item.id, kind)}
          />
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
          name={
            item.worst === 'pass' && item.state !== 'failed' ? 'check-circle' : 'warning-circle'
          }
          className={cn(
            'flex-none',
            item.worst === 'pass' && item.state !== 'failed' ? 'text-sage' : 'text-clay-text'
          )}
        />
        <span className="min-w-0 truncate" title={reviewPrompt(item)}>
          {reviewPrompt(item)}
        </span>
        <span className="ml-auto" />
        <Button icon="arrow-clockwise" disabled={isBusy} onClick={() => void reprocess(item.id)}>
          Reprocess
        </Button>
        <Button variant="ghost" icon="arrow-square-out" onClick={() => void openInBlender(item.id)}>
          Open in Blender
        </Button>
        <Button
          variant="primary"
          icon="check"
          disabled={isBusy || !project || item.state === 'failed'}
          title={
            !project
              ? 'Create a project first'
              : item.link && item.link.project === project.id
                ? `Adds a version to ${item.link.assetName}`
                : `Moves it into ${project.name}`
          }
          onClick={() => project && void accept(item.id, project.id)}
        >
          Accept to Blender
        </Button>
      </div>
    </div>
  );
}
