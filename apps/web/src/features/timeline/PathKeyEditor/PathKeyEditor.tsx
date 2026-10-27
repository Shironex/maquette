import { Button, KeyValueList, NumberField, Select } from '@maquette/ui';
import type { Easing } from '@/ipc/client';
import { usePathKeyEditor } from './PathKeyEditor.hooks';

/**
 * The selected energize key: how much of the path is lit at this stop (0
 * is dark, 100 lit from end to end), and how the fill eases between keys.
 */
export default function PathKeyEditor() {
  const e = usePathKeyEditor();
  if (!e) return null;
  return (
    <section aria-label="Selected energize key" className="flex flex-col gap-3.5">
      <h2 className="text-panel-title">{e.title}</h2>
      <NumberField
        label={`Lit at ${e.stopName}`}
        unit="%"
        min={0}
        max={100}
        value={e.amount}
        onChange={e.setAmount}
        fieldClassName="w-[140px]"
      />
      <Select<Easing> label="Easing" options={e.easings} value={e.easing} onChange={e.setEasing} />
      <KeyValueList items={e.facts} />
      {!e.keyed ? (
        <p className="text-meta text-fg-mute">
          Not keyed at {e.stopName} yet. Set how much is lit to key it.
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          icon="trash"
          disabled={!e.keyed}
          disabledReason="Nothing is keyed at this stop"
          tooltip="Removes the key at this stop"
          onClick={e.removeKey}
        >
          Remove this key
        </Button>
        <Button
          size="sm"
          variant="ghost"
          tooltip="Removes every key of this path's fill"
          onClick={e.removeTrack}
        >
          Remove the track
        </Button>
        <Button
          size="sm"
          variant="ghost"
          icon="path"
          tooltip="Goes to the Inspector's Paths tab to change the path itself"
          onClick={e.openPath}
        >
          Edit the path
        </Button>
      </div>
    </section>
  );
}
