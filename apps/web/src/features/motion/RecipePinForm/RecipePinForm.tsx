import { Icon, NumberField, SegmentedControl, Select, cn } from '@maquette/ui';
import type { PinFlat } from '@/ipc/client';
import { useRecipePinForm } from './RecipePinForm.hooks';

const whole = (v: number) => String(Math.round(v));

/** A note box under the pin's settings. */
function Note({
  icon,
  title,
  detail,
  warn,
}: {
  icon: 'check-circle' | 'warning' | 'info';
  title: string;
  detail: string;
  warn?: boolean;
}) {
  return (
    <div
      className={cn(
        'rounded-card border px-2.5 py-2',
        warn ? 'border-amber/60 bg-amber/10' : 'border-line bg-panel'
      )}
    >
      <p className="flex items-center gap-1.5 text-[12.5px] font-medium">
        <Icon name={icon} size={14} className={warn ? 'text-amber' : 'text-fg-dim'} />
        {title}
      </p>
      <p className="mt-0.5 text-meta text-fg-mute">{detail}</p>
    </div>
  );
}

/**
 * A pin's own settings, under its kind, weight and range: the hand or foot,
 * where it stays (where it is at its first frame, or a blank), what of a
 * foot keeps flat, its eases, and whether the preview's limb can stay on
 * the spot.
 */
export default function RecipePinForm({ id }: { id: string }) {
  const f = useRecipePinForm(id);
  if (!f) return null;
  return (
    <div role="group" aria-label={f.title} className="flex flex-col gap-2.5">
      <div className="flex items-baseline gap-2">
        <h4 className="flex-1 text-[13px] font-semibold">{f.title}</h4>
        <span className="font-mono text-[11.5px] text-fg-mute">{f.frames}</span>
      </div>
      <SegmentedControl
        label="Bone"
        tone="clay"
        block
        options={f.bones}
        value={f.layer.bone}
        onChange={f.setBone}
      />
      <Select label="Stays at" size="sm" options={f.places} value={f.place} onChange={f.setPlace} />
      {f.foot ? (
        <SegmentedControl
          label="Keep flat"
          tone="clay"
          block
          options={f.flats}
          value={f.flat}
          onChange={(v: PinFlat) => f.setFlat(v)}
        />
      ) : null}
      <div className="grid grid-cols-2 gap-2">
        <NumberField
          label="Eases in"
          size="sm"
          unit="f"
          min={0}
          max={600}
          format={whole}
          value={f.easeIn}
          onChange={f.setEaseIn}
        />
        <NumberField
          label="Eases out"
          size="sm"
          unit="f"
          min={0}
          max={600}
          format={whole}
          value={f.easeOut}
          onChange={f.setEaseOut}
        />
      </div>
      {f.check ? (
        <Note
          icon={f.checkOk ? 'check-circle' : 'warning'}
          title={f.check.title}
          detail={f.check.detail}
          warn={!f.checkOk}
        />
      ) : null}
      {f.foot ? null : (
        <Note
          icon="info"
          title="Hand pins take a blank"
          detail="Pin a hand to a point or a prop blank, such as a wall or a shaft, so the recipe travels."
        />
      )}
    </div>
  );
}
