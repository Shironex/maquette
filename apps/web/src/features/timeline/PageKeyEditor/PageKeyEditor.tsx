import { Button, KeyValueList, NumberField, Select } from '@maquette/ui';
import type { Easing, TrackLook } from '@/ipc/client';
import { usePageKeyEditor } from './PageKeyEditor.hooks';

/**
 * The selected key of the render style or hero text: how far in the look or
 * how shown the copy is at this stop, and how it eases between keys.
 */
export default function PageKeyEditor() {
  const e = usePageKeyEditor();
  if (!e) return null;
  return (
    <section aria-label="Selected page key" className="flex flex-col gap-3.5">
      <h2 className="text-panel-title">{e.title}</h2>
      {e.style ? (
        <Select<TrackLook> label="Look" options={e.looks} value={e.look} onChange={e.setLook} />
      ) : null}
      <NumberField
        label={e.amountLabel}
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
          Not keyed at {e.stopName} yet. Set an amount to key it.
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
          tooltip="Removes this whole track"
          onClick={e.removeTrack}
        >
          Remove the track
        </Button>
      </div>
    </section>
  );
}
