import { Button, SegmentedControl } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useMirrorForm } from './MirrorForm.hooks';

const HANDS = [
  { value: 'right', label: 'Right handed' },
  { value: 'left', label: 'Left handed' },
] as const;

/** The mirror: right or left handed, and swapping the changes' sides with it. */
export default function MirrorForm() {
  const f = useMirrorForm();
  const swaps = f.swaps.map(s => <li key={s.key}>{s.line}</li>);
  return (
    <div
      role="group"
      aria-label="Mirror settings"
      className="flex flex-col gap-2.5 border-t border-line px-3 py-3"
    >
      <SegmentedControl
        label="Mirror"
        options={HANDS}
        value={f.on ? 'left' : 'right'}
        onChange={v => f.set(v === 'left')}
      />
      <p className="text-meta text-fg-mute">
        {f.on
          ? 'The whole motion plays mirrored across the model’s middle: left and right swap.'
          : 'Turn it on for a left handed copy of the motion.'}
      </p>
      {f.swaps.length ? (
        <div>
          <Button size="sm" variant="ghost" icon="arrows-left-right" onClick={f.swapNow}>
            Swap sides in the changes
          </Button>
        </div>
      ) : null}
      {f.asking !== null ? (
        <ChoiceDialog
          title={f.asking ? 'Swap sides in the changes too?' : 'Swap the changes back?'}
          onDismiss={f.cancel}
          choices={[
            {
              label: 'Keep the bones',
              variant: 'ghost',
              onClick: () => f.turn(Boolean(f.asking), false),
            },
            { label: 'Swap sides', onClick: () => f.turn(Boolean(f.asking), true) },
          ]}
        >
          <p className="text-[13px] text-fg-dim">
            These changes name a side. With the sides swapped they stay on the same hand as the
            motion moves to the other one.
          </p>
          <ul className="mt-2 list-disc pl-5 font-mono text-[12px]">{swaps}</ul>
        </ChoiceDialog>
      ) : null}
    </div>
  );
}
