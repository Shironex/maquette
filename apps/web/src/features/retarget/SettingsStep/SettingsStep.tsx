import { Button, NumberField, SegmentedControl, TextField, Toggle, cn } from '@maquette/ui';
import { formatBytes } from '@/lib/format';
import { playLine, trimmed } from '@/state/retarget';
import { useSettingsStep } from './SettingsStep.hooks';

const MOTION = [
  { value: 'place', label: 'In place' },
  { value: 'root', label: 'Root motion' },
] as const;

/** Step three: how each clip goes onto this model. Never saved back to the library. */
export default function SettingsStep() {
  const step = useSettingsStep();
  if (!step.sheet) return null;
  const { selected } = step;
  const rows = step.rows.map(({ setting, clip }) => {
    const on = selected?.setting.clip === setting.clip;
    return (
      <tr key={setting.clip} className={cn('border-b border-line', on && 'bg-clay-soft/50')}>
        <td className="py-1.5">
          <button
            type="button"
            aria-pressed={on}
            onClick={() => step.select(setting.clip)}
            className="cursor-pointer text-left text-[13px]"
          >
            {clip.name}
          </button>
          {step.outdated.includes(setting.clip) ? (
            <span className="ml-2 rounded-pill bg-amber-soft px-1.5 text-[11px] text-amber">
              update
            </span>
          ) : null}
        </td>
        <td className="py-1.5 font-mono text-[12px]">{setting.name}</td>
        <td className="py-1.5 font-mono text-[12px] text-fg-dim">
          {setting.start} to {setting.end}
          {trimmed(setting, clip) ? ' · trim' : ''}
        </td>
        <td className="py-1.5 text-meta text-fg-dim">{setting.loops ? 'loop' : ''}</td>
        <td className="py-1.5 text-meta text-fg-dim">
          {setting.rootMotion ? 'root motion' : 'in place'}
        </td>
        <td className="py-1.5 font-mono text-[12px] text-fg-dim">
          {(setting.speed ?? 1).toFixed(2)}x
        </td>
        <td className="py-1.5 font-mono text-[12px] text-fg-mute">
          {formatBytes(clip.bytes ?? 0)}
        </td>
      </tr>
    );
  });
  return (
    <>
      <section aria-label="Clip settings" className="flex min-w-0 flex-1 flex-col gap-3 p-4">
        <div className="flex items-center justify-between gap-3">
          <p className="text-meta text-fg-dim">
            Names are made from the clip name in snake_case, pack words dropped. Change any of them.
          </p>
          <Button size="sm" onClick={step.resetNames}>
            Reset names
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          <table className="w-full border-collapse text-left">
            <thead className="text-caption tracking-[0.04em] text-fg-mute uppercase">
              <tr className="border-b border-line">
                <th className="py-2 font-normal">Library clip</th>
                <th className="py-2 font-normal">Name in {step.sheet.plan?.name ?? 'the model'}</th>
                <th className="py-2 font-normal">Frames</th>
                <th className="py-2 font-normal">Loop</th>
                <th className="py-2 font-normal">Motion</th>
                <th className="py-2 font-normal">Speed</th>
                <th className="py-2 font-normal">Size</th>
              </tr>
            </thead>
            <tbody>{rows}</tbody>
          </table>
        </div>
      </section>
      {selected ? (
        <aside
          aria-label={selected.clip.name}
          className="flex w-[300px] flex-none flex-col gap-3 overflow-y-auto border-l border-line p-4"
        >
          <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">
            {selected.clip.name}
          </p>
          <TextField
            label="Name in the model"
            mono
            value={selected.setting.name}
            onChange={e => step.edit({ name: e.target.value })}
          />
          <SegmentedControl
            label="Motion"
            options={MOTION}
            value={selected.setting.rootMotion ? 'root' : 'place'}
            onChange={v => step.edit({ rootMotion: v === 'root' })}
          />
          <Toggle
            label="Loop"
            checked={selected.setting.loops}
            onChange={loops => step.edit({ loops })}
          />
          <Toggle
            label="Pin the feet"
            description="Holds each foot still while it is planted, for a clip whose feet slide."
            checked={selected.setting.pinFeet ?? false}
            onChange={pinFeet => step.edit({ pinFeet })}
          />
          <div className="grid grid-cols-2 gap-2">
            <NumberField
              label="Trim in"
              value={selected.setting.start}
              min={0}
              max={selected.clip.frames}
              onChange={start => step.edit({ start })}
            />
            <NumberField
              label="Trim out"
              value={selected.setting.end}
              min={0}
              max={selected.clip.frames}
              onChange={end => step.edit({ end })}
            />
          </div>
          <NumberField
            label="Speed"
            unit="x"
            value={selected.setting.speed ?? 1}
            min={0.25}
            max={4}
            onChange={speed => step.edit({ speed })}
          />
          <p className="font-mono text-[12px] text-fg-dim">
            {playLine(selected.setting, selected.clip)}
          </p>
          <p className="mt-auto text-meta text-fg-mute">
            Runs headless in Blender and makes v{(step.sheet.plan?.version ?? 0) + 1} for review; v
            {step.sheet.plan?.version ?? 0} stays.
          </p>
        </aside>
      ) : null}
    </>
  );
}
