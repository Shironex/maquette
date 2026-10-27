import { Button, KeyValueList, NumberField, SegmentedControl, Select } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { useStateKeyEditor } from './StateKeyEditor.hooks';

type Driver = 'stop' | 'scroll' | 'time';

/**
 * The selected state change: arriving at a camera stop plays the move
 * once, a scroll range scrubs it with the page. The move's easing and
 * timing are set in the Motion view.
 */
export default function StateKeyEditor() {
  const e = useStateKeyEditor();
  if (!e) return null;
  const drivers = [
    { value: 'stop' as const, label: 'Camera stop' },
    { value: 'scroll' as const, label: 'Scroll range' },
    {
      value: 'time' as const,
      label: 'Time',
      disabled: true,
      hint: 'Not built yet: a timed change starts from a camera stop',
    },
  ];
  return (
    <section aria-label="Selected change" className="flex flex-col gap-3.5">
      <h2 className="text-panel-title">{e.title}</h2>
      <div className="flex flex-col gap-1.5">
        <SectionLabel>Driven by</SectionLabel>
        <SegmentedControl<Driver>
          label="Driven by"
          tone="clay"
          options={drivers}
          value={e.kind}
          onChange={d => d !== 'time' && e.setKind(d)}
        />
      </div>
      <Select label="Goes to" options={e.states} value={e.to} onChange={e.setTo} />
      {e.kind === 'stop' ? (
        <Select label="At the stop" options={e.stops} value={e.stop} onChange={e.setStop} />
      ) : (
        <div className="flex gap-2.5">
          <NumberField
            label="Starts"
            unit="%"
            min={0}
            max={99}
            value={e.start}
            onChange={e.setStart}
            fieldClassName="w-[96px]"
          />
          <NumberField
            label="Ends"
            unit="%"
            min={1}
            max={100}
            value={e.end}
            onChange={e.setEnd}
            fieldClassName="w-[96px]"
          />
        </div>
      )}
      <KeyValueList items={e.facts} />
      <p className="text-meta text-fg-mute">
        The move&apos;s easing and length are set in the Motion view.
      </p>
      <div>
        <Button
          size="sm"
          icon="trash"
          tooltip="Removes this change from the timeline"
          onClick={e.remove}
        >
          Remove this change
        </Button>
      </div>
    </section>
  );
}
