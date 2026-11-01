import { Suspense } from 'react';
import { StageLoader, useStageReady } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
import { Button, EmptyState, Icon, SegmentedControl, Select, cn } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { BakeBackNote } from '../BakeBackNote';
import { formatK } from '@/lib/format';
import { badge } from '../badge';
import { PartActions, PartsList } from './PartsList';
import { MaterialPanel } from '../MaterialPanel';
import { PartsCompare } from '../PartsCompare';
import {
  usePartsModel,
  usePartsPanel,
  usePartsSheets,
  type PartsMode,
  type PartsPage,
} from './PartsPanel.hooks';

const ModelPreview = lazyAfterPaint(() => import('@/features/viewer/ModelPreview'));

const MODES: readonly { value: PartsMode; label: string }[] = [
  { value: 'shaded', label: 'Shaded' },
  { value: 'parts', label: 'Colour by part' },
  { value: 'wireframe', label: 'Wireframe' },
];

const PAGES: readonly { value: PartsPage; label: string }[] = [
  { value: 'parts', label: 'Parts' },
  { value: 'compare', label: 'Before and after' },
];

const CARD = 'flex flex-col gap-1.5 rounded-card border border-line bg-panel p-3';

/** One model's parts: select them in the viewer, rename, join and hide them. */
export default function PartsPanel() {
  const p = usePartsPanel();
  const { openSplit, openAssemble, openInBlender, openMotion } = usePartsSheets();
  const preview = useStageReady(p.view.url);
  const measured = usePartsModel(p.asset?.id ?? null, p.found?.version ?? null);
  if (!p.asset) {
    return (
      <div className="grid flex-1 place-items-center">
        <EmptyState
          icon="stack"
          title="No models yet"
          body="Accept a model from the Inbox first. Its parts show here."
        />
      </div>
    );
  }
  const found = p.found;
  const tweak =
    measured.tweaking && measured.model && measured.project
      ? { project: measured.project, model: measured.model }
      : null;
  const stem = found?.name.toLowerCase().replace(/[^a-z0-9]+/g, '_') ?? '';
  const oneMesh = found !== null && found.rows.length < 2;
  const { url, mode, setMode, look, selectedNames, hiddenNames } = p.view;
  const modelOptions = p.models.map(a => ({ value: a.id, label: a.name }));
  const chosenRows = p.chosen.map(r => (
    <li key={r.id} className="flex justify-between gap-2">
      <span className="truncate font-mono text-fg">{r.name}</span>
      <span className="flex-none text-fg-mute">
        {r.materials.join(', ')} · {formatK(r.triangles)} tris
      </span>
    </li>
  ));
  return (
    <div className="flex min-h-0 flex-1 gap-5">
      <section className="flex min-w-0 flex-1 flex-col gap-3">
        <div className="flex items-center gap-3">
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-panel-title">
              {p.asset.name}
              {found?.version ? (
                <span className="ml-2 font-mono text-fg-mute">v{found.version}</span>
              ) : null}
            </h2>
            <p className="text-meta text-fg-mute">
              {found
                ? oneMesh
                  ? 'One mesh: it moves only as a whole'
                  : `${found.rows.length} parts${found.split ? ' · split in Blender' : ''}`
                : p.busy
                  ? p.splitting
                    ? 'Splitting a copy in Blender...'
                    : 'Reading the parts...'
                  : ' '}
            </p>
          </div>
          {found?.version ? (
            <Button
              size="sm"
              icon="arrow-square-out"
              onClick={() => void openInBlender(found)}
              tooltip={`Opens v${found.version} in Blender's window to look closer.`}
            >
              Open in Blender
            </Button>
          ) : null}
          <SegmentedControl
            label="Parts page"
            options={PAGES}
            value={p.page}
            onChange={p.setPage}
          />
          {p.page === 'compare' && measured.compareVersions ? (
            <Button size="sm" onClick={measured.compareVersions}>
              Provider vs provider
            </Button>
          ) : null}
          {p.page === 'parts' ? (
            <SegmentedControl label="Parts view" options={MODES} value={mode} onChange={setMode} />
          ) : null}
        </div>
        {p.page === 'compare' && found ? (
          <PartsCompare asset={p.asset} found={found} onChanged={p.reload} />
        ) : (
          <>
            <div className="relative min-h-0 flex-1 overflow-hidden rounded-[14px] border border-line bg-studio">
              {url ? (
                <CanvasBoundary resetKey={url}>
                  <Suspense fallback={<StageLoader />}>
                    <ModelPreview
                      url={url}
                      mode={mode}
                      parts={mode === 'parts' ? look : undefined}
                      onPickPart={(part, adding) => p.pick(part, adding)}
                      onLoaded={preview.onLoaded}
                      className="absolute inset-0"
                    />
                  </Suspense>
                </CanvasBoundary>
              ) : null}
              {url && !preview.ready ? <StageLoader /> : null}
              {!url && p.busy ? (
                <StageLoader
                  label={p.splitting ? 'Splitting a copy in Blender...' : 'Reading the parts...'}
                  estimate={p.estimate}
                />
              ) : null}
              {mode === 'parts' && selectedNames.length ? (
                <span className={cn(badge, 'pointer-events-none top-3 left-3')}>
                  {selectedNames.join(' and ')} selected
                </span>
              ) : null}
              {mode === 'parts' && hiddenNames.length ? (
                <span className={cn(badge, 'pointer-events-none top-3 right-3 text-fg-mute')}>
                  <Icon name="cube-transparent" size={12} />
                  {hiddenNames.join(', ')} hidden
                </span>
              ) : null}
              {url ? (
                <span className={cn(badge, 'pointer-events-none bottom-3 left-3')}>
                  <Icon name="cursor-click" size={12} />
                  Click a part to select it · Shift adds
                </span>
              ) : null}
            </div>
            {found && !oneMesh ? <WhereNamesGo stem={stem} p={p} /> : null}
          </>
        )}
      </section>
      <aside aria-label="Parts" className="flex w-[300px] flex-none flex-col gap-3 overflow-y-auto">
        <Select
          label="Selected model"
          options={modelOptions}
          value={p.asset.id}
          onChange={p.pickAsset}
        />
        {p.error ? (
          <p role="alert" className="text-meta text-danger">
            {p.error}
          </p>
        ) : null}
        {found ? <BakeBackNote project={found.project} asset={found.asset} /> : null}
        {oneMesh ? (
          <div className={CARD}>
            <h3 className="font-semibold">One mesh, no parts</h3>
            <p className="text-meta text-fg-mute">
              Motion states and explode tracks move named parts. Split it in Blender, free and on a
              copy, or let Tripo or Meshy find the parts.
            </p>
            <Button
              icon="stack"
              loading={p.busy}
              tooltip="Free in Blender, or on Tripo or Meshy"
              onClick={() =>
                openSplit({
                  project: found.project,
                  asset: found.asset,
                  version: found.version,
                  item: null,
                  inBlender: p.edit.addSplit,
                })
              }
            >
              Split into parts...
            </Button>
          </div>
        ) : (
          <div className={CARD}>
            <h3 className="flex items-baseline justify-between font-semibold">
              Parts
              <span className="text-meta font-normal text-fg-mute">double click to rename</span>
            </h3>
            <PartActions
              count={p.selected.length}
              allHidden={p.edit.allHidden}
              onMerge={p.edit.merge}
              onHide={p.edit.hide}
            />
            <PartsList
              rows={p.rows}
              selected={p.selected}
              onPick={p.pick}
              onRename={p.edit.rename}
              onUnmerge={p.edit.unmerge}
            />
            <p className="text-meta text-fg-mute">
              Renames carry into Motion states, Timeline tracks and the export. Hiding is only for
              looking: a hidden part still ships, as a ghost here.
            </p>
          </div>
        )}
        {p.chosen.length ? (
          <div className={CARD}>
            <h3 className="flex items-center justify-between font-semibold">
              Selected parts
              {measured.model?.tweakAsset ? (
                <Button
                  size="sm"
                  variant="ghost"
                  aria-expanded={measured.tweaking}
                  onClick={measured.toggleTweaking}
                >
                  Tweak materials
                </Button>
              ) : null}
            </h3>
            <ul className="flex flex-col gap-1 text-meta">{chosenRows}</ul>
            {tweak ? (
              <>
                <MaterialPanel
                  project={tweak.project}
                  model={tweak.model}
                  only={p.chosen.flatMap(r => r.materials)}
                />
                <p className="text-meta text-fg-mute">
                  A tweak goes by material, so every part that shares it changes with it.
                </p>
              </>
            ) : null}
          </div>
        ) : null}
        {p.page === 'compare' ? (
          <div className={CARD}>
            <h3 className="font-semibold">What keeping does</h3>
            <p className="text-meta text-fg-mute">
              The kept version becomes current on the Board and in the next export. Motion states
              that name parts need the split version; keeping the one mesh parks them.
            </p>
          </div>
        ) : null}
        {found ? (
          <div className={CARD}>
            <h3 className="font-semibold">One prop from part models</h3>
            <p className="text-meta text-fg-mute">
              Made the parts as models of their own, from object refs? Join them into one prop, a
              node per part under one name.
            </p>
            <Button
              icon="package"
              onClick={() => openAssemble({ project: found.project, parts: [found.asset] })}
            >
              Assemble a prop...
            </Button>
          </div>
        ) : null}
        {found ? (
          <div className="mt-auto flex gap-2">
            <Button
              className="flex-1"
              icon="export"
              disabled={!measured.model}
              disabledReason="Its numbers are still loading"
              loading={measured.exporting}
              onClick={measured.export}
            >
              Export GLB
            </Button>
            <Button
              className="flex-1"
              icon="film-strip"
              tooltip="Its clips and states, in Motion"
              onClick={() => openMotion(found.asset)}
            >
              Open in Motion
            </Button>
          </div>
        ) : null}
      </aside>
    </div>
  );
}

/** Where the part names go: the export's node names and what moves them. */
function WhereNamesGo({ stem, p }: { stem: string; p: ReturnType<typeof usePartsPanel> }) {
  const nodes = p.shipped.join(', ');
  const uses = p.uses.map(u => (
    <li key={u.label} className="flex justify-between gap-2">
      <span className="truncate text-fg">{u.label}</span>
      <span className="flex-none font-mono text-fg-mute">{u.parts}</span>
    </li>
  ));
  return (
    <div className="grid flex-none grid-cols-2 gap-3">
      <div className={CARD}>
        <h3 className="flex items-center gap-1.5 font-semibold">
          <Icon name="export" size={14} />
          Export
          <span className="font-mono text-meta font-normal text-fg-mute">{stem}.glb</span>
        </h3>
        <p className="text-meta text-fg-mute">
          Nodes: <span className="font-mono text-fg">{nodes}</span>
        </p>
      </div>
      <div className={CARD}>
        <h3 className="flex items-center gap-1.5 font-semibold">
          <Icon name="film-strip" size={14} />
          {p.game ? 'Used by' : 'Motion states'}
        </h3>
        {p.uses.length ? (
          <ul className="flex flex-col gap-0.5 text-meta">{uses}</ul>
        ) : (
          <p className="text-meta text-fg-mute">
            {p.game
              ? 'The game picks the parts by these node names.'
              : 'None yet. Motion states and explode tracks pick parts by name.'}
          </p>
        )}
      </div>
    </div>
  );
}
