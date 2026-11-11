import { Button, NumberField, SegmentedControl, Select } from '@maquette/ui';
import { GripPicture } from '@/features/common/GripPicture';
import type { GripHands } from '@/ipc/client';
import { counted } from '@/lib/format';
import { boneSide } from '@/state/recipe-grips';
import { RecipeNote } from '../RecipeNote';
import { useRecipeGripForm } from './RecipeGripForm.hooks';

const whole = (v: number) => String(Math.round(v));
const CAPTION = 'text-[12px] leading-4 font-medium text-fg-dim';
const HANDS: { value: GripHands; label: string }[] = [
  { value: 'both', label: 'Both' },
  { value: 'right', label: 'Right' },
  { value: 'left', label: 'Left' },
];

/**
 * A grip's own settings, under its weight and range: the library grip it
 * was copied from, its picture, the hands, the blank it holds, its eases,
 * and the offer to take a newer revision, never forced.
 */
export default function RecipeGripForm({ id }: { id: string }) {
  const f = useRecipeGripForm(id);
  if (!f) return null;
  const pose = f.layer.pose ?? {};
  return (
    <div role="group" aria-label={`Grip ${f.title}`} className="flex flex-col gap-2.5">
      <div className="flex items-baseline gap-2">
        <h4 className="min-w-0 flex-1 truncate text-[13px] font-semibold">{f.title}</h4>
        <span className="font-mono text-[11.5px] text-fg-mute">{counted(f.bones, 'bone')}</span>
      </div>
      <div className="grid h-[110px] place-items-center rounded-card border border-line bg-studio p-2">
        <GripPicture
          pose={pose}
          holds={boneSide(f.layer.bone)}
          label={`${f.title}, as the recipe holds it`}
        />
      </div>
      <div className="flex flex-col gap-1">
        <span aria-hidden className={CAPTION}>
          Hands
        </span>
        <SegmentedControl<GripHands>
          label="Hands"
          tone="clay"
          block
          options={HANDS}
          value={f.hands}
          onChange={f.setHands}
        />
      </div>
      <Select label="Holds" size="sm" options={f.targets} value={f.target} onChange={f.setTarget} />
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
      {f.note?.kind === 'newer' ? (
        <RecipeNote
          icon="warning"
          warn
          title={`${f.note.name} has r${f.note.latest}`}
          detail={`This recipe holds r${f.current}. Updating takes the new pose; where it sits, its eases and its weight stay. Or keep r${f.current}.`}
          actions={
            <Button size="sm" loading={f.updating} onClick={f.update}>
              Update to r{f.note.latest}
            </Button>
          }
        />
      ) : f.note?.kind === 'gone' ? (
        <RecipeNote
          icon="info"
          title="Its library grip is gone"
          detail="The recipe keeps its own copy, so it still bakes as it is."
        />
      ) : null}
      {f.refused ? <p className="text-meta text-danger">Not updated: {f.refused}</p> : null}
    </div>
  );
}
