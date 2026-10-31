import { Button, Icon, NumberField, Select, Toggle } from '@maquette/ui';
import type { PackSource, RenderLook } from '@/ipc/client';
import {
  LOOK_PAPER,
  LOOK_OPTIONS,
  STYLE_LABEL,
  secondsAbout,
  isLook,
  shortPath,
  fileOf,
} from '@/state/renders';
import { useRenderOptions } from './RenderOptions.hooks';

export interface RenderOptionsProps {
  source: PackSource;
  assetId: string;
}

const SIZES = ['1024', '2048', '4096'].map(v => ({ value: v, label: `${v} px` }));
const HOW: Record<RenderLook, string> = {
  blueprint: 'Workbench with Freestyle lines',
  toon: 'Workbench flat colour with ink lines',
  clay: 'Workbench studio light with cavity',
  shaded: 'Cycles at a low sample count',
};

function Colour({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (next: string) => void;
}) {
  return (
    <label className="flex items-center gap-2">
      <span className="flex-1">{label}</span>
      <span className="font-mono text-meta text-fg-mute uppercase">{value}</span>
      <input
        type="color"
        aria-label={label}
        value={value}
        onChange={e => onChange(e.target.value)}
        className="size-7 cursor-pointer rounded-control border border-line-strong bg-transparent"
      />
    </label>
  );
}

/** The settings of the style on show, where it goes, and Render. */
export default function RenderOptions({ source, assetId }: RenderOptionsProps) {
  const o = useRenderOptions(source, assetId);
  const { style, settings, change } = o;
  const look = isLook(style) ? style : settings.look;
  const file = fileOf(style, settings);
  return (
    <aside
      aria-label="Style settings"
      className="flex flex-none flex-col gap-3 rounded-card border border-line bg-panel p-3.5"
    >
      <div className="flex items-center justify-between">
        <h3 className="text-panel-title">{STYLE_LABEL[style]}</h3>
        <Button size="sm" variant="ghost" onClick={o.reset}>
          Reset
        </Button>
      </div>
      {style === 'blueprint' ? (
        <>
          <Colour
            label="Lines"
            value={settings.lineColor}
            onChange={lineColor => change({ lineColor })}
          />
          <Colour
            label="Paper"
            value={settings.paperColor ?? LOOK_PAPER.blueprint}
            onChange={paperColor => change({ paperColor })}
          />
        </>
      ) : null}
      {style === 'clay' ? (
        <Colour
          label="Clay"
          value={settings.clayColor}
          onChange={clayColor => change({ clayColor })}
        />
      ) : null}
      {isLook(style) && style !== 'blueprint' ? (
        <Colour
          label={style === 'shaded' ? 'Backdrop' : 'Paper'}
          value={settings.paperColor ?? LOOK_PAPER[style]}
          onChange={paperColor => change({ paperColor })}
        />
      ) : null}
      {style === 'blueprint' || style === 'toon' ? (
        <div className="grid grid-cols-2 gap-2">
          <NumberField
            label="Line weight"
            unit="px"
            min={0.5}
            max={8}
            value={settings.lineWeight ?? 1.6}
            onChange={lineWeight => change({ lineWeight })}
          />
          <NumberField
            label="Edges from crease"
            unit="°"
            min={1}
            max={120}
            value={settings.crease}
            onChange={crease => change({ crease })}
          />
        </div>
      ) : null}
      {style === 'blueprint' ? (
        <>
          <Toggle
            label="Hidden lines, dashed"
            checked={settings.hiddenDashed}
            onChange={hiddenDashed => change({ hiddenDashed })}
          />
          <Toggle
            label="Height and width marks"
            checked={settings.marks}
            onChange={marks => change({ marks })}
          />
          <Toggle
            label="Title block"
            checked={settings.titleBlock}
            onChange={titleBlock => change({ titleBlock })}
          />
        </>
      ) : null}
      {style === 'contact-sheet' ? (
        <Select<RenderLook>
          label="Style"
          value={settings.look}
          options={LOOK_OPTIONS}
          onChange={l => change({ look: l })}
        />
      ) : null}
      <div className="flex flex-col gap-2.5 border-t border-line pt-3">
        <h4 className="text-meta font-medium tracking-wide text-fg-mute uppercase">Output</h4>
        <div className="grid grid-cols-2 gap-2">
          <Select
            label="Size"
            value={String(settings.size)}
            options={SIZES}
            onChange={size => change({ size: Number(size) })}
          />
          <div className="flex flex-col gap-1">
            <span className="font-medium">Format</span>
            <span className="py-1.5 font-mono text-fg-dim">PNG</span>
          </div>
        </div>
        {isLook(style) ? (
          <Toggle
            label="Transparent paper"
            checked={settings.transparent}
            onChange={transparent => change({ transparent })}
          />
        ) : null}
        <div>
          <p className="text-meta text-fg-mute">Saved to</p>
          <p className="font-mono text-meta break-all text-fg-dim">
            {o.dir ? shortPath(o.dir, o.folder, file) : file}
          </p>
        </div>
      </div>
      <p className="flex gap-1.5 text-meta text-fg-mute">
        <Icon name="info" size={13} className="mt-0.5 flex-none" />
        Headless Blender, {HOW[look]}. No credits, about {secondsAbout(style, settings)} s at{' '}
        {settings.size} px.
      </p>
      <div className="flex justify-end gap-2">
        <Button icon="stack" onClick={o.addToBatch}>
          Add to batch
        </Button>
        <Button variant="primary" icon="image" onClick={o.render}>
          Render
        </Button>
      </div>
    </aside>
  );
}
