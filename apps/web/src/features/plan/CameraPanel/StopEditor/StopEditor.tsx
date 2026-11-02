import { Button, NumberField, TextField } from '@maquette/ui';
import { useStopEditor } from './StopEditor.hooks';

const AXES = ['X', 'Y', 'Z'] as const;

/**
 * The selected stop's settings in the camera plan: name, where the camera
 * stands and where it looks (metres), field of view, and its place in the
 * main line with buttons to move it earlier or later, or take it out.
 */
export default function StopEditor({ nodeId }: { nodeId: string }) {
  const v = useStopEditor(nodeId);
  if (!v.stop) return null;
  const stop = v.stop;
  const row = (key: 'position' | 'target', label: string) => {
    const fields = AXES.map((axis, i) => (
      <NumberField
        key={axis}
        label={`${label} ${axis}`}
        size="sm"
        unit="m"
        value={stop[key][i] ?? 0}
        min={-1_000_000}
        max={1_000_000}
        onChange={value => v.axis(key, i, value)}
      />
    ));
    return (
      <fieldset className="flex flex-col gap-1">
        <legend className="pb-1 text-[11.5px] font-medium text-fg-dim">{label}</legend>
        <div className="grid grid-cols-3 gap-1.5">{fields}</div>
      </fieldset>
    );
  };
  return (
    <section aria-label={`Stop ${stop.name}`} className="flex flex-col gap-2.5">
      <h3 className="text-[13px] font-semibold text-fg">
        {v.place ? `Stop ${v.place} of ${v.count}` : v.by ? `Proposed by ${v.by}` : 'Set aside'}
      </h3>
      <TextField
        label="Name"
        value={v.name}
        maxLength={120}
        onChange={e => v.setName(e.target.value)}
        onBlur={v.commitName}
        onKeyDown={e => {
          if (e.key === 'Enter') v.commitName();
        }}
      />
      {row('position', 'Camera at')}
      {row('target', 'Looks at')}
      <NumberField
        label="Field of view"
        size="sm"
        unit="°"
        value={stop.fov ?? 35}
        min={5}
        max={120}
        onChange={v.fov}
      />
      <div className="flex flex-wrap gap-1.5">
        {v.place ? (
          <>
            <Button
              size="sm"
              icon="arrow-left"
              disabled={v.place === 1}
              disabledReason="It is the first stop"
              onClick={v.earlier}
            >
              Earlier
            </Button>
            <Button
              size="sm"
              icon="arrow-right"
              disabled={v.place === v.count}
              disabledReason="It is the last stop"
              onClick={v.later}
            >
              Later
            </Button>
          </>
        ) : null}
        <Button
          size="sm"
          variant="ghost"
          icon="trash"
          tooltip="Takes the stop out of the plan; the stops around it join. Undo brings it back."
          onClick={v.remove}
        >
          Remove
        </Button>
      </div>
    </section>
  );
}
