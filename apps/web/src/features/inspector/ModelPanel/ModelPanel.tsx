import {
  Button,
  Icon,
  SegmentedControl,
  Select,
  VersionBadge,
  budgetScale,
  cn,
  type SegmentedOption,
  type SelectOption,
} from '@maquette/ui';
import type {
  CompressPreset,
  InspectModel,
  InspectView,
  MeshoptLevel,
  TextureCodec,
} from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import {
  budgetRows,
  budgetTargetNote,
  formatMb,
  modelSubtitle,
  num,
  presetNote,
  statCards,
  textureLabel,
  textureLine,
  textureTone,
} from '@/state/inspector';
import { withGroupCodec } from '@/state/materials';
import { Hairline } from '../Hairline';
import { MapCodecRows } from '../MapCodecRows';
import { MaterialPanel } from '../MaterialPanel';
import { PrepPanel } from '../PrepPanel';
import { LibraryPanel } from '../LibraryPanel';
import { SectionTitle } from '../SectionTitle';

const CODECS: SegmentedOption<TextureCodec>[] = [
  { value: 'etc1s', label: 'KTX2 ETC1S' },
  { value: 'uastc', label: 'UASTC' },
];
const MESHOPT: SelectOption<MeshoptLevel>[] = [
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
];

export interface ModelPanelProps {
  view: InspectView;
  model: InspectModel | null;
  thumb: string | null;
  /** The stem whose budget row is highlighted. */
  onPickBudget: (stem: string) => void;
  /** Saves the model's preset row; null while presets cannot be saved. */
  onPreset: ((row: CompressPreset) => void) | null;
  /** Re-exports the model through the export step; null where that cannot work. */
  onExport?: ((model: InspectModel) => void) | null;
  /** Export GLB is running for this model. */
  exporting?: boolean;
  /**
   * In a game: the selected model's folder budget in triangles (null when its
   * folder has none). A game ships no compression and has no scene total.
   */
  game?: { triangles: number | null } | null;
}

/** Models the pipeline's export step makes, so Export GLB can make them again. */
function exportable(model: InspectModel): boolean {
  return model.source === 'raw' || model.source === 'shipped';
}

export default function ModelPanel({
  view,
  model,
  thumb,
  onPickBudget,
  onPreset,
  onExport,
  exporting,
  game,
}: ModelPanelProps) {
  const trisLimit = game ? (game.triangles ?? Infinity) : view.limits.trisPerModel;
  const trisWhy = game
    ? `Over its folder's ${trisLimit.toLocaleString('en-US')} triangle budget`
    : `Over the ${trisLimit.toLocaleString('en-US')} triangle budget per model`;
  const rows = budgetRows(view);
  const totalMb = num(view.scene.bytes) / 1e6;
  const sceneMb = num(view.limits.sceneMb);
  const overTotal = totalMb > sceneMb;
  const save = (patch: Partial<Pick<CompressPreset, 'codec' | 'mapCodecs' | 'meshopt'>>) => {
    if (!model || !onPreset) return;
    const p = model.preset;
    onPreset({
      model: model.model,
      label: model.label,
      textureSize: p.textureSize,
      dataTextureSize: p.dataTextureSize,
      codec: patch.codec ?? p.codec,
      mapCodecs: patch.mapCodecs ?? p.mapCodecs,
      meshopt: patch.meshopt ?? p.meshopt,
      lock: null,
    });
  };
  const locked = Boolean(model?.preset.lock);
  const cards =
    model && !model.error
      ? statCards(model, trisLimit).map(card => (
          <div key={card.label} className="rounded-card border border-line bg-raised px-2.5 py-2">
            <div className="text-meta text-fg-mute">{card.label}</div>
            <div
              className={cn(
                'font-mono text-[15px] leading-5 tabular-nums',
                card.over && 'text-danger'
              )}
              title={
                card.label === 'GPU memory'
                  ? 'An estimate: textures as the GPU keeps them, with mipmaps, plus vertices and indices.'
                  : card.over
                    ? trisWhy
                    : undefined
              }
            >
              {card.value}
            </div>
          </div>
        ))
      : null;
  const textures = model
    ? model.textures.map((t, i) => (
        <div key={`${t.name}-${i}`} className="flex items-center gap-2.5">
          <span
            className="size-[22px] flex-none rounded-[5px]"
            style={{ background: textureTone(t) }}
          />
          <span className="min-w-0 flex-1 truncate" title={`${t.name}, ${t.codec}`}>
            {textureLabel(t, model.textures)}
          </span>
          <span className="font-mono text-[12px] whitespace-nowrap text-fg-mute">
            {textureLine(t)}
          </span>
        </div>
      ))
    : null;
  const budgets = rows.map(row => {
    const scale = budgetScale(row.mb, row.targetMb);
    const on = model?.model === row.model;
    return (
      <button
        key={row.model}
        type="button"
        title={budgetTargetNote(row, sceneMb)}
        onClick={() => onPickBudget(row.model)}
        aria-pressed={on}
        className={cn(
          '-mx-2 flex cursor-pointer flex-col gap-[5px] rounded-control px-2 py-1 text-left hover:bg-hover',
          on && 'bg-hover'
        )}
      >
        <span className="flex w-full justify-between">
          <span>{row.label}</span>
          <span
            className={cn(
              'font-mono text-[12px] tabular-nums',
              row.over ? 'text-danger' : 'text-fg'
            )}
          >
            {row.target === 'scene' ? (
              <>
                {formatMb(row.mb)} MB{' '}
                <span className="font-sans text-meta text-fg-mute">of the scene target</span>
              </>
            ) : (
              <>
                {formatMb(row.mb)}{' '}
                <span className="text-fg-mute">
                  / {formatMb(row.targetMb)} MB
                  {row.target === 'share' ? (
                    <span className="font-sans text-meta"> share</span>
                  ) : null}
                </span>
              </>
            )}
          </span>
        </span>
        <span
          role="meter"
          aria-label={`${row.label} transfer`}
          aria-valuenow={Math.round(row.mb * 100) / 100}
          aria-valuemin={0}
          aria-valuemax={row.targetMb}
          className="relative block h-1 w-full rounded-pill bg-hover"
        >
          <span
            className={cn(
              'absolute inset-y-0 left-0 rounded-pill',
              row.over ? 'bg-danger' : 'bg-sage'
            )}
            style={{ width: `${scale.fill}%` }}
          />
          <span
            className="absolute -inset-y-1 -ml-px w-0.5 rounded-[1px] bg-fg shadow-[0_0_0_2px_var(--color-panel)]"
            style={{ left: `${scale.targetAt}%` }}
          />
        </span>
      </button>
    );
  });
  return (
    <aside
      aria-label="Model details"
      className="flex w-[340px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel px-5 py-[18px]"
    >
      <div className="flex flex-col gap-2.5">
        <SectionTitle>Selected model</SectionTitle>
        {model ? (
          <>
            <div className="flex items-center gap-3">
              <span className="grid size-12 flex-none place-items-center overflow-hidden rounded-[8px] bg-studio">
                {thumb ? (
                  <img src={thumb} alt="" className="size-full object-contain" />
                ) : (
                  <Icon name="cube" className="text-fg-mute" />
                )}
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="truncate text-[15px] leading-5 font-semibold tracking-[-0.01em]">
                    {model.label}
                  </span>
                  {model.version ? <VersionBadge version={model.version} /> : null}
                </div>
                <div className="truncate font-mono text-[12px] text-fg-mute">
                  {modelSubtitle(model)}
                </div>
              </div>
            </div>
            {model.error ? (
              <p className="rounded-[8px] bg-danger-soft px-2.5 py-2 text-meta text-danger">
                {model.error}
              </p>
            ) : (
              <div className="grid grid-cols-2 gap-2">{cards}</div>
            )}
            {onExport && exportable(model) ? (
              <Button
                size="sm"
                icon="export"
                loading={exporting}
                disabled={exporting}
                onClick={() => onExport(model)}
                className="self-start"
                tooltip="Runs the export step for this model only and replaces its raw GLB"
              >
                {exporting ? 'Exporting' : 'Export GLB'}
              </Button>
            ) : null}
          </>
        ) : (
          <p className="text-fg-mute">
            Click a model in the view, or a budget row below, to see its numbers.
          </p>
        )}
      </div>

      {model && !model.error ? (
        <>
          {game ? null : (
            <>
              <Hairline />
              <div className="flex flex-col gap-2.5">
                <SectionTitle>Compression</SectionTitle>
                <div className="flex items-center justify-between gap-3">
                  <span className="text-fg-dim">Texture codec</span>
                  <SegmentedControl
                    label="Texture codec"
                    tone="clay"
                    options={CODECS}
                    value={model.preset.codec}
                    onChange={codec => {
                      if (!locked && codec !== model.preset.codec) save({ codec });
                    }}
                    disabled={locked}
                  />
                </div>
                <div className="flex items-center justify-between gap-3">
                  <span className="text-fg-dim">Meshopt</span>
                  <div className="flex items-center gap-2">
                    {locked ? <Icon name="lock-simple" size={16} className="text-fg-mute" /> : null}
                    <Select
                      aria-label="Meshopt level"
                      size="sm"
                      inline
                      options={MESHOPT}
                      value={model.preset.meshopt}
                      disabled={locked || !onPreset}
                      onChange={meshopt => save({ meshopt })}
                      fieldClassName="w-[120px]"
                    />
                  </div>
                </div>
                <MapCodecRows
                  model={model}
                  onCodec={
                    locked || !onPreset
                      ? null
                      : (group, codec) =>
                          save({ mapCodecs: withGroupCodec(model.preset.mapCodecs, group, codec) })
                  }
                />
                <p className="rounded-[8px] bg-raised px-2.5 py-2 text-meta text-fg-mute">
                  {presetNote(model)}
                </p>
              </div>
            </>
          )}

          <Hairline />
          <MaterialPanel project={view.project} model={model} />

          <Hairline />
          <PrepPanel project={view.project} model={model} />

          <Hairline />
          <LibraryPanel project={view.project} model={model} />

          <Hairline />
          <div className="flex flex-col gap-2">
            <SectionTitle>Textures</SectionTitle>
            {model.textures.length === 0 ? <p className="text-fg-mute">No textures.</p> : textures}
          </div>
        </>
      ) : null}

      <Hairline />
      {game ? (
        <p className="text-meta text-fg-mute">
          Each model checks its triangles and texture size against its folder&apos;s budget in the
          Budget tab. There is no scene total, and the game export keeps its own compression.
        </p>
      ) : (
        <>
          <div className="flex flex-col gap-[9px]">
            <SectionTitle aside="transfer">Model budgets</SectionTitle>
            {rows.length === 0 ? (
              <p className="text-fg-mute">Run the pipeline to see what each model ships at.</p>
            ) : (
              budgets
            )}
          </div>

          <div className="mt-auto flex items-center justify-between border-t border-line pt-3">
            <span className="text-fg-dim">Scene total</span>
            <span
              className={cn(
                'font-mono text-[13px] tabular-nums',
                overTotal ? 'text-danger' : 'text-fg'
              )}
              title={`${view.scene.models} models, ${formatBytes(num(view.scene.bytes))}`}
            >
              {formatMb(totalMb)} <span className="text-fg-mute">/ {formatMb(sceneMb)} MB</span>
            </span>
          </div>
        </>
      )}
    </aside>
  );
}
