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
} from '@maquette/ui';
import type { AssetKind, InboxItem } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatAgo, formatBytes } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/app-store';
import { useInboxStore } from '@/state/inbox-store';
import {
  checkSummary,
  firstToReview,
  itemChip,
  itemMeta,
  reviewPrompt,
  sortChecks,
} from '@/state/inbox';
import type { PreviewMode } from '@/features/viewer/ModelPreview';

const ModelPreview = lazy(() => import('@/features/viewer/ModelPreview'));

const MODES: readonly { value: PreviewMode; label: string }[] = [
  { value: 'shaded', label: 'Shaded' },
  { value: 'wireframe', label: 'Wireframe' },
  { value: 'uv', label: 'UV' },
  { value: 'bones', label: 'Bones' },
];

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
            <h2 className="text-screen-title">Model inbox</h2>
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
        {item ? <InboxDetail key={item.id} item={item} /> : <InboxEmpty dir={snapshot?.dir} />}
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
      <span className="grid size-[52px] flex-none place-items-center rounded-[8px] bg-studio text-fg-mute">
        <Icon name={item.kind === 'character' ? 'cube-focus' : 'cube'} size={22} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-semibold">{item.name}</span>
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
  const project = useAppStore(s => s.workspace?.project ?? null);
  const { busy, reprocess, accept, openInBlender, setKind } = useInboxStore();
  const report = item.report;
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
    ...(report
      ? [
          {
            label: 'Meshes',
            value: `${report.meshes} · ${report.primitives} primitives`,
            mono: true,
          },
          { label: 'Materials', value: String(report.materials), mono: true },
          ...(report.generator ? [{ label: 'Made by', value: report.generator }] : []),
        ]
      : []),
    { label: 'File', value: `inbox/${item.id}`, mono: true },
  ];

  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1">
        <section className="flex min-w-0 flex-1 flex-col gap-3.5 px-6 pt-[18px] pb-5">
          <div className="flex items-center gap-3">
            <div className="min-w-0 flex-1">
              <h2 className="text-screen-title truncate">{item.name}</h2>
              <p className="mt-0.5 text-fg-mute">
                arrived {formatAgo(item.modifiedMs ?? now, now)}
              </p>
            </div>
            <SegmentedControl
              label="Preview mode"
              options={MODES.filter(m => m.value !== 'bones' || hasBones)}
              value={mode}
              onChange={setMode}
            />
          </div>
          <div className="relative min-h-0 flex-1 overflow-hidden rounded-[14px] border border-line bg-[#7f8083]">
            {url && item.state === 'ready' ? (
              <Suspense fallback={null}>
                <ModelPreview
                  url={url}
                  mode={mode}
                  onError={setLoadError}
                  className="absolute inset-0"
                />
              </Suspense>
            ) : null}
            {!url || loadError || item.state === 'failed' ? (
              <div className="absolute inset-0 grid place-items-center text-[#2a2926]">
                <span className="rounded-full bg-black/10 px-3 py-1 text-meta">
                  {loadError ?? item.error ?? 'No preview for this file'}
                </span>
              </div>
            ) : null}
            <span className="pointer-events-none absolute top-3 left-3 rounded-full bg-black/35 px-2.5 py-1 text-meta text-white/90">
              {MODES.find(m => m.value === mode)?.label}
            </span>
            <span className="pointer-events-none absolute right-3 bottom-3 rounded-full bg-black/35 px-2.5 py-1 text-meta text-white/90">
              Drag to orbit · scroll to zoom
            </span>
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
            hint={
              item.kindGuessed ? 'A guess from the file. Pick one to change the checks.' : undefined
            }
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
        <span className="min-w-0 truncate">{reviewPrompt(item)}</span>
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
          title={project ? `Moves it into ${project.name}` : 'Create a project first'}
          onClick={() => project && void accept(item.id, project.id)}
        >
          Accept to Blender
        </Button>
      </div>
    </div>
  );
}
