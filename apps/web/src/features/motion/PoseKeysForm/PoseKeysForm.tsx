import { Button, Icon, NumberField, SegmentedControl, cn } from '@maquette/ui';
import type { KeyEase, LayerMode } from '@/ipc/client';
import { counted } from '@/lib/format';
import { boneWords } from '@/state/recipe-bones';
import { EASES, EASE_ABOUT, EASE_WORDS } from '@/state/recipe-keys';
import { BoneMap } from './BoneMap';
import { usePoseKeysForm } from './PoseKeysForm.hooks';

const AXES = ['X', 'Y', 'Z'] as const;
const oneDecimal = (v: number) => String(Number(v.toFixed(1)));
const EASE_OPTIONS = EASES.map(e => ({ value: e, label: EASE_WORDS[e], hint: EASE_ABOUT[e] }));
const MODE_OPTIONS: { value: LayerMode; label: string; hint: string }[] = [
  { value: 'add', label: 'Add to the clip', hint: 'Turns on top of what the clip does' },
  { value: 'replace', label: 'Replace it', hint: 'Turns from the rest pose, in place of the clip' },
];

/**
 * A pose key change's settings: the bone picker, the picked bone's turn at
 * the playhead's frame (keying it there), how the motion eases into that
 * key, how the keys are laid on and how much, copy, mirror and remove, and
 * the joint limits line.
 */
export default function PoseKeysForm({ id }: { id: string }) {
  const f = usePoseKeysForm(id);
  if (!f.layer) return null;
  const name = boneWords(f.bone);
  const fields = AXES.map((axis, i) => (
    <NumberField
      key={axis}
      label={`Turn ${axis}`}
      size="sm"
      unit="°"
      min={-180}
      max={180}
      format={oneDecimal}
      value={f.turn[i]!}
      onChange={v => f.setAxis(i, v)}
      fieldClassName="flex-1"
    />
  ));
  const where = f.dt && !f.dt.plain ? ` · clip ${f.spot.clip + 1}, frame ${f.spot.frame}` : '';
  const over = f.hits.length > 0;
  return (
    <div
      role="group"
      aria-label={`Change ${f.layer.id} settings`}
      className="flex flex-col gap-3 border-t border-line px-3 py-3"
    >
      <BoneMap bone={f.bone} counts={f.counts} onPick={f.pickBone} />
      <div className="flex flex-col gap-2">
        <div className="flex items-baseline justify-between gap-2">
          <h3 className="text-meta font-semibold text-fg">Pose keys, {name.toLowerCase()}</h3>
          <span className="font-mono text-[11.5px] text-fg-dim">
            Frame {f.frame}
            {where}
          </span>
        </div>
        <p className="text-meta text-fg-mute">
          {f.keyedHere
            ? `${name} has a key on this frame.`
            : f.here
              ? `Frame ${f.frame} has a key; ${name.toLowerCase()} is not in it yet.`
              : 'No key on this frame. Turning a bone keys it here.'}
        </p>
        <div className="flex gap-2">{fields}</div>
        <div className="flex flex-col gap-1">
          <span className="text-meta text-fg-dim">Eases into this key</span>
          <SegmentedControl<KeyEase>
            label="Eases into this key"
            tone="clay"
            block
            options={EASE_OPTIONS}
            value={f.ease}
            onChange={f.setEase}
          />
        </div>
        <div className="grid grid-cols-[1fr_auto] items-end gap-2">
          <div className="flex flex-col gap-1">
            <span className="text-meta text-fg-dim">Laid on</span>
            <SegmentedControl<LayerMode>
              label="Laid on"
              tone="clay"
              block
              options={MODE_OPTIONS}
              value={f.mode}
              onChange={f.setMode}
            />
          </div>
          <NumberField
            label="Weight"
            size="sm"
            unit="%"
            min={0}
            max={100}
            format={v => String(Math.round(v))}
            value={Math.round(f.weight * 100)}
            onChange={v => f.setWeight(v / 100)}
            fieldClassName="w-20"
          />
        </div>
        <div className="flex flex-wrap gap-1.5">
          <Button size="sm" variant="secondary" icon="plus" onClick={f.key}>
            Key {name.toLowerCase()}
          </Button>
          <Button size="sm" variant="ghost" icon="copy" onClick={f.copy}>
            Copy pose
          </Button>
          {f.copied ? (
            <Button size="sm" variant="ghost" onClick={f.paste}>
              Paste pose
            </Button>
          ) : null}
          <Button size="sm" variant="ghost" onClick={f.mirror}>
            Mirror pose
          </Button>
          <Button
            size="sm"
            variant="ghost"
            icon="hand-pointing"
            disabled={Boolean(f.gripBlocked)}
            disabledReason={f.gripBlocked}
            tooltip={`Saves the hands at frame ${f.frame} to the grip library`}
            onClick={f.saveGrip}
          >
            Save a grip
          </Button>
          <Button
            size="sm"
            variant="ghost"
            icon="trash"
            disabled={!f.here}
            onClick={() => f.remove(true)}
          >
            Remove key
          </Button>
        </div>
      </div>
      <div
        className={cn(
          'flex items-start gap-2 rounded-card border px-2.5 py-2',
          over ? 'border-amber/60 bg-amber/10' : 'border-line bg-raised'
        )}
      >
        <Icon
          name={over ? 'warning' : 'check'}
          size={14}
          className={cn('mt-0.5 flex-none', over ? 'text-amber' : 'text-sage')}
        />
        <div className="flex flex-col gap-0.5 text-meta">
          <span className="font-semibold text-fg">
            {over ? 'Past the joint limits' : 'Inside the joint limits'}
          </span>
          <span className="text-fg-dim">
            {f.limits ?? `${name} has no limits to keep, or no turn here.`}
          </span>
          {over ? (
            <span className="text-fg-mute">It only warns; the bake review shows it too.</span>
          ) : null}
        </div>
      </div>
      <p className="text-meta text-fg-mute">
        Only what you touched is saved: {counted(f.bones.length, 'bone')},{' '}
        {counted(f.layer.keys?.length ?? 0, 'key')}. The rest of each frame comes from the clip.
        {f.hidden ? ` ${counted(f.hidden, 'key')} sit in frames a trim cuts out.` : ''}
      </p>
      {f.problem ? (
        <p role="alert" className="flex items-start gap-1.5 text-meta text-danger">
          <Icon name="warning-circle" size={14} className="mt-0.5 flex-none" />
          {f.problem}
        </p>
      ) : null}
      <div>
        <Button size="sm" variant="ghost" icon="trash" onClick={f.removeChange}>
          Remove change
        </Button>
      </div>
    </div>
  );
}
