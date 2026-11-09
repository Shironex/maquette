import { Button, NumberField, SegmentedControl, Select } from '@maquette/ui';
import type { TimeKind } from '@/ipc/client';
import { V2_LIMITS } from '@/state/recipe-v2-check';
import { useTimeForm } from './TimeForm.hooks';

const whole = (v: number) => String(Math.round(v));
const two = (v: number) => String(Number(v.toFixed(2)));

/** A retime's range, speed and ease, with how many frames it plays in. */
function RetimeFields({ f }: { f: NonNullable<ReturnType<typeof useTimeForm>> }) {
  const { edit } = f;
  return (
    <>
      <div className="grid grid-cols-2 gap-2">
        <NumberField
          label="From"
          size="sm"
          unit="f"
          min={0}
          max={(edit.to ?? f.clip.frames) - 1}
          format={whole}
          value={edit.from ?? 0}
          onChange={v => f.patch({ from: Math.round(v) })}
        />
        <NumberField
          label="To"
          size="sm"
          unit="f"
          min={(edit.from ?? 0) + 1}
          max={f.clip.frames}
          format={whole}
          value={edit.to ?? 0}
          onChange={v => f.patch({ to: Math.round(v) })}
        />
        <NumberField
          label="Speed"
          size="sm"
          unit="x"
          min={V2_LIMITS.minSpeed}
          max={V2_LIMITS.maxSpeed}
          format={two}
          value={edit.speed ?? 1}
          onChange={v => f.patch({ speed: v })}
        />
        <NumberField
          label="Ease"
          size="sm"
          unit="f"
          min={0}
          max={60}
          format={whole}
          value={edit.ease ?? 0}
          onChange={v => f.patch({ ease: Math.round(v) })}
        />
      </div>
      <SegmentedControl
        label="Speed presets"
        options={f.presets}
        value={f.preset}
        onChange={v => f.patch({ speed: Number(v) })}
      />
      <p className="font-mono text-[11.5px] text-fg-dim">{`${f.plays} · ${f.length}`}</p>
      <p className="text-meta text-fg-mute">
        Faster than 1x plays the range in fewer frames, slower stretches it. Ease ramps the speed in
        and out over that many frames so the change does not jump.
      </p>
    </>
  );
}

/** A freeze or hold over: where, how long, how much it settles, and its eases. */
function HoldFields({ f }: { f: NonNullable<ReturnType<typeof useTimeForm>> }) {
  const { edit } = f;
  const most =
    edit.kind === 'freeze' ? V2_LIMITS.holdFrames : Math.max(f.clip.frames - (edit.at ?? 0), 1);
  return (
    <>
      <div className="grid grid-cols-2 gap-2">
        <NumberField
          label="At"
          size="sm"
          unit="f"
          min={0}
          max={Math.max(f.clip.frames - 1, 0)}
          format={whole}
          value={edit.at ?? 0}
          onChange={v => f.patch({ at: Math.round(v) })}
        />
        <NumberField
          label="Frames"
          size="sm"
          unit="f"
          min={1}
          max={most}
          format={whole}
          value={edit.frames ?? 1}
          onChange={v => f.patch({ frames: Math.round(v) })}
        />
        <NumberField
          label="Settle"
          size="sm"
          unit="%"
          min={0}
          max={100}
          format={whole}
          value={Math.round((edit.breathe ?? 0) * 100)}
          onChange={v => f.patch({ breathe: Math.round(v) / 100 })}
        />
        <span />
        <NumberField
          label="Ease in"
          size="sm"
          unit="f"
          min={0}
          max={60}
          format={whole}
          value={edit.easeIn ?? 0}
          onChange={v => f.patch({ easeIn: Math.round(v) })}
        />
        <NumberField
          label="Ease out"
          size="sm"
          unit="f"
          min={0}
          max={60}
          format={whole}
          value={edit.easeOut ?? 0}
          onChange={v => f.patch({ easeOut: Math.round(v) })}
        />
      </div>
      <p className="font-mono text-[11.5px] text-fg-dim">{f.length}</p>
      <p className="text-meta text-fg-mute">
        {`${f.note} Settle keeps a little breath in the chest while it holds.`}
      </p>
    </>
  );
}

/** One time edit's settings: kind, clip and its own fields, and remove. */
export default function TimeForm({ index }: { index: number }) {
  const f = useTimeForm(index);
  if (!f) return null;
  return (
    <div
      role="group"
      aria-label={`Time edit ${index + 1} settings`}
      className="flex flex-col gap-2.5 border-t border-line px-3 py-3"
    >
      <div className="grid grid-cols-2 gap-2">
        <Select
          label="Kind"
          size="sm"
          options={f.kinds}
          value={f.edit.kind}
          onChange={(k: TimeKind) => f.setKind(k)}
        />
        <Select
          label="Clip"
          size="sm"
          options={f.clips}
          value={String(f.edit.clip)}
          onChange={f.setClip}
        />
      </div>
      {f.edit.kind === 'retime' ? <RetimeFields f={f} /> : <HoldFields f={f} />}
      {f.problem ? (
        <p role="alert" className="text-meta text-danger">
          {f.problem}
        </p>
      ) : null}
      <div>
        <Button size="sm" variant="ghost" icon="trash" onClick={f.remove}>
          Remove time edit
        </Button>
      </div>
    </div>
  );
}
