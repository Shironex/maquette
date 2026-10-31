import { Checkbox, Select, TextField, Toggle, cn } from '@maquette/ui';
import { FilterGroup, FilterList, FilterOption } from '@/features/common/FilterList';
import { RIG_LABEL, lengthLine } from '@/state/animations';
import { formatBytes } from '@/lib/format';
import GameNote from '../GameNote';
import RoughPreview from '@/features/viewer/RoughPreview';
import { useChooseStep } from './ChooseStep.hooks';

/** Step one: the model, then the clips, with what they add to it. */
export default function ChooseStep() {
  const step = useChooseStep();
  if (!step.sheet) return null;
  const { sheet, totals } = step;
  const modelOptions = step.models.map(m => ({
    value: m.asset,
    label: `${m.name} · v${m.version}`,
  }));
  const packs = step.packs.map(([pack, count]) => (
    <FilterOption
      key={pack}
      label={pack}
      count={count}
      on={sheet.pack === pack}
      onClick={() => step.change({ pack: sheet.pack === pack ? null : pack })}
    />
  ));
  const tags = step.tags.map(([tag, count]) => (
    <FilterOption
      key={tag}
      label={tag}
      count={count}
      on={sheet.tag === tag}
      onClick={() => step.change({ tag: sheet.tag === tag ? null : tag })}
    />
  ));
  const rows = step.shown.map(({ clip }) => {
    const on = sheet.chosen.includes(clip.id);
    const marks = [clip.loops ? 'loop' : null, clip.rootMotion ? 'root' : null].filter(Boolean);
    return (
      <li
        key={clip.id}
        className={cn(
          'grid grid-cols-[minmax(0,1fr)_150px_70px_64px] items-center gap-3 border-b border-line px-3 py-1.5',
          on && 'bg-clay-soft/50'
        )}
      >
        <Checkbox
          checked={on}
          onChange={() => step.toggle(clip.id)}
          className="min-w-0"
          label={
            <span className="block min-w-0">
              <span className="block truncate text-[13px]">{clip.name}</span>
              <span className="block truncate text-meta text-fg-mute" aria-hidden="true">
                {clip.pack}
              </span>
            </span>
          }
        />
        <span className="font-mono text-[11.5px] text-fg-dim">
          {[lengthLine(clip), ...marks].join(' · ')}
        </span>
        <span className="font-mono text-[11.5px] text-fg-mute">{formatBytes(clip.bytes ?? 0)}</span>
        <span className="text-meta text-fg-mute">{RIG_LABEL[clip.rig.kind]}</span>
      </li>
    );
  });
  const chosenPacks = totals.packs.map(([pack, n]) => (
    <li key={pack} className="flex justify-between gap-2 text-meta">
      <span className="truncate text-fg-dim">{pack}</span>
      <span className="font-mono text-fg-mute">{n}</span>
    </li>
  ));
  const modelBytes = step.model?.bytes ?? 0;
  const modelName = step.model?.name ?? 'the model';
  return (
    <>
      <aside className="flex w-[250px] flex-none flex-col gap-3 overflow-y-auto border-r border-line p-4">
        {modelOptions.length ? (
          <Select
            label="Model"
            options={modelOptions}
            value={sheet.asset ?? undefined}
            onChange={step.setAsset}
          />
        ) : (
          <p className="text-meta text-fg-mute">
            This project has no rigged model yet. Rig one, then add clips to it.
          </p>
        )}
        <TextField
          label="Search"
          icon="magnifying-glass"
          value={sheet.query}
          onChange={e => step.change({ query: e.target.value })}
          placeholder="Clip, pack or tag"
        />
        <FilterList>
          <FilterGroup title="Packs">
            <FilterOption
              label="All clips"
              count={step.total}
              on={sheet.pack === null}
              onClick={() => step.change({ pack: null })}
            />
            {packs}
          </FilterGroup>
          {tags.length ? <FilterGroup title="Tags">{tags}</FilterGroup> : null}
        </FilterList>
        <Toggle
          label="Humanoid rigs only"
          description={`${step.humanoid} ${step.humanoid === 1 ? 'clip' : 'clips'} a humanoid map can read`}
          checked={sheet.humanoidOnly}
          onChange={humanoidOnly => step.change({ humanoidOnly })}
        />
      </aside>
      <section aria-label="Library clips" className="min-w-0 flex-1 overflow-y-auto">
        <ul>{rows}</ul>
        {rows.length ? null : (
          <p className="p-6 text-meta text-fg-mute">
            No clip matches. Clear the search or pick another pack.
          </p>
        )}
      </section>
      <aside
        aria-label="Chosen clips"
        className="flex w-[300px] flex-none flex-col gap-3 overflow-y-auto border-l border-line p-4"
      >
        <section aria-label="Rough preview" className="flex flex-col gap-1.5">
          <p className="text-[13px] font-semibold">Rough preview on {modelName}</p>
          <RoughPreview
            project={sheet.project}
            asset={sheet.asset}
            modelName={modelName}
            modelFile={step.model?.file ?? null}
            clip={step.previewClip}
            picked={sheet.picked}
            className="aspect-[4/3] w-full flex-none"
          />
          <p className="text-meta text-fg-mute">
            Played live with the saved bone map. The bake in Blender is the exact one.
          </p>
        </section>
        <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">
          Chosen · {totals.clips}
        </p>
        <p className="font-mono text-[12px] text-fg-dim">{totals.frames} frames</p>
        <ul className="flex flex-col gap-1">{chosenPacks}</ul>
        <p className="text-meta text-fg-dim">
          {totals.rootMotion} with root motion · {totals.loops} looping
        </p>
        <div className="rounded-card border border-line p-3">
          <p className="text-[13px] font-semibold">What it adds to {modelName}</p>
          <p className="mt-1 text-meta text-fg-dim">
            {formatBytes(modelBytes)} to about {formatBytes(modelBytes + totals.bytes)}: new motion,
            resampled, about {formatBytes(totals.bytes)}. Triangles and textures stay as they are.
          </p>
        </div>
        <GameNote asset={sheet.asset} name={modelName} adds={totals.bytes} />
        <p className="text-meta text-fg-mute">Nothing is baked until Review.</p>
      </aside>
    </>
  );
}
