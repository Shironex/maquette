import { Button, Icon, NumberField, Select } from '@maquette/ui';
import type { LayerKind } from '@/ipc/client';
import { LIMITS } from '@/state/recipe-check';
import { num } from '@/state/recipe-draft';
import { RecipePinForm } from '../RecipePinForm';
import { RecipeRange } from '../RecipeRange';
import { useRecipeChangeForm } from './RecipeChangeForm.hooks';

const AXES = ['X', 'Y', 'Z'] as const;
const decimals = (places: number) => (v: number) => String(Number(v.toFixed(places)));

/** One change's settings: kind, bone, target, weight, where it plays and what its kind reads. */
export default function RecipeChangeForm({ id }: { id: string }) {
  const f = useRecipeChangeForm(id);
  const { layer } = f;
  if (!layer) return null;
  const s = layer.settings;
  const targeted = layer.kind === 'reach' || layer.kind === 'look-at';
  const triple = (what: 'rotation' | 'offset') =>
    AXES.map((axis, i) => (
      <NumberField
        key={axis}
        label={`${what === 'rotation' ? 'Turn' : 'Move'} ${axis}`}
        size="sm"
        unit={what === 'rotation' ? '°' : 'm'}
        min={what === 'rotation' ? -LIMITS.rotation : -LIMITS.offset}
        max={what === 'rotation' ? LIMITS.rotation : LIMITS.offset}
        format={decimals(what === 'rotation' ? 1 : 3)}
        value={num((what === 'rotation' ? s.rotation : s.offset)?.[i])}
        onChange={v => (what === 'rotation' ? f.setRotation(i, v) : f.setOffset(i, v))}
        fieldClassName="flex-1"
      />
    ));
  return (
    <div
      role="group"
      aria-label={`Change ${layer.id} settings`}
      className="flex flex-col gap-2.5 border-t border-line px-3 py-3"
    >
      <div className="grid grid-cols-2 gap-2">
        <Select
          label="Kind"
          size="sm"
          options={f.kinds}
          value={layer.kind}
          onChange={(k: LayerKind) => f.setKind(k)}
        />
        {layer.kind === 'pin' ? null : (
          <Select
            label="Bone"
            size="sm"
            options={f.bones}
            value={layer.bone}
            onChange={f.setBone}
          />
        )}
        {targeted ? (
          <Select
            label="Target"
            size="sm"
            options={f.targets}
            value={s.target ?? ''}
            onChange={f.setTarget}
          />
        ) : null}
        <NumberField
          label="Weight"
          size="sm"
          unit="%"
          min={0}
          max={100}
          format={decimals(0)}
          value={Math.round(num(s.weight, 1) * 100)}
          onChange={v => f.setWeight(v / 100)}
        />
      </div>
      <RecipeRange
        frames={f.frames}
        convert={f.convert}
        start={num(s.start)}
        end={num(s.end, 1)}
        onChange={f.setRange}
      />
      {layer.kind === 'pin' ? <RecipePinForm id={layer.id} /> : null}
      {layer.kind === 'nudge' ? (
        <>
          <div className="flex gap-2">{triple('rotation')}</div>
          <div className="flex gap-2">{triple('offset')}</div>
        </>
      ) : null}
      {layer.kind === 'wave' ? (
        <div className="flex gap-2">
          <NumberField
            label="Waves"
            size="sm"
            min={0.1}
            max={LIMITS.cycles}
            format={decimals(1)}
            value={num(s.cycles)}
            onChange={f.setCycles}
            fieldClassName="flex-1"
          />
          <NumberField
            label="Swing"
            size="sm"
            unit="°"
            min={1}
            max={LIMITS.amount}
            format={decimals(0)}
            value={num(s.amount)}
            onChange={f.setAmount}
            fieldClassName="flex-1"
          />
        </div>
      ) : null}
      {f.problem ? (
        <p role="alert" className="flex items-start gap-1.5 text-meta text-danger">
          <Icon name="warning-circle" size={14} className="mt-0.5 flex-none" />
          {f.problem}
        </p>
      ) : null}
      <div>
        <Button size="sm" variant="ghost" icon="trash" onClick={f.remove}>
          Remove change
        </Button>
      </div>
    </div>
  );
}
