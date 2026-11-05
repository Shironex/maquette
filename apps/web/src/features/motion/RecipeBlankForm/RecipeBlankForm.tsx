import { Button, Icon, NumberField, SegmentedControl, Select, TextField } from '@maquette/ui';
import type { BlankKind } from '@/ipc/client';
import { LIMITS } from '@/state/recipe-check';
import { BLANK_WORDS, num } from '@/state/recipe-draft';
import { useRecipeBlankForm } from './RecipeBlankForm.hooks';

const AXES = ['X', 'Y', 'Z'] as const;
const KINDS = (['prop', 'point'] as const).map(k => ({ value: k, label: BLANK_WORDS[k] }));
const metres = (v: number) => String(Number(v.toFixed(3)));

/** One blank: what it is called, whether a prop or a point fills it, and what does. */
export default function RecipeBlankForm({ id }: { id: string }) {
  const f = useRecipeBlankForm(id);
  const { blank } = f;
  if (!blank) return null;
  const point = AXES.map((axis, i) => (
    <NumberField
      key={axis}
      label={axis}
      size="sm"
      unit="m"
      min={-LIMITS.point}
      max={LIMITS.point}
      format={metres}
      value={num(blank.point?.[i])}
      onChange={v => f.setPoint(i, v)}
      fieldClassName="flex-1"
    />
  ));
  return (
    <div
      role="group"
      aria-label={`Blank ${blank.id} settings`}
      className="flex flex-col gap-2.5 border-t border-line px-3 py-3"
    >
      <TextField
        label="What fills it"
        value={blank.label}
        maxLength={LIMITS.label}
        onChange={e => f.setLabel(e.target.value)}
      />
      <SegmentedControl
        label="Filled with"
        options={KINDS}
        value={blank.kind}
        onChange={(k: BlankKind) => f.setKind(k)}
      />
      {blank.kind === 'prop' ? (
        <Select
          label="Prop"
          size="sm"
          options={f.props}
          value={blank.prop ?? ''}
          onChange={f.setProp}
        />
      ) : (
        <>
          <div className="flex gap-2">{point}</div>
          <p className="text-meta text-fg-mute">
            Metres from where the model stands: X to its left, Y up, Z the way it faces.
          </p>
        </>
      )}
      {f.gone ? (
        <p className="text-meta text-fg-mute">
          {f.cleared
            ? `Saving empties it for good: ${f.gone} will no longer fill it.`
            : `The file still names ${f.gone}, which is no longer an asset: restoring it fills this blank again. Leave open empties it for good.`}
        </p>
      ) : null}
      {f.problem ? (
        <p role="alert" className="flex items-start gap-1.5 text-meta text-danger">
          <Icon name="warning-circle" size={14} className="mt-0.5 flex-none" />
          {f.problem}
        </p>
      ) : null}
      <div className="flex gap-2">
        {f.cleared ? (
          <Button size="sm" variant="ghost" icon="arrow-counter-clockwise" onClick={f.keepGone}>
            Keep {f.gone}
          </Button>
        ) : (
          <Button size="sm" variant="ghost" icon="x" onClick={f.clear}>
            Leave open
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          icon="trash"
          tooltip={
            f.aiming
              ? `${f.aiming === 1 ? 'A change aims' : `${f.aiming} changes aim`} at it and will need a new target`
              : undefined
          }
          onClick={f.remove}
        >
          Remove blank
        </Button>
      </div>
    </div>
  );
}
