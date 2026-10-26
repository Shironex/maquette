import { Button, KeyValueList, NumberField, Select } from '@maquette/ui';
import type { Easing } from '@/ipc/client';
import { useExplodeKeyEditor } from './ExplodeKeyEditor.hooks';

/**
 * The selected explode key: how far apart the parts are at this stop (0
 * is together, 100 is the full spread), and the track's spread and easing.
 * Each part moves out from the model's centre.
 */
export default function ExplodeKeyEditor() {
  const e = useExplodeKeyEditor();
  if (!e) return null;
  return (
    <section aria-label="Selected explode key" className="flex flex-col gap-3.5">
      <h2 className="text-panel-title">{e.title}</h2>
      <div className="flex gap-2.5">
        <NumberField
          label={`Apart at ${e.stopName}`}
          unit="%"
          min={0}
          max={100}
          value={e.amount}
          onChange={e.setAmount}
          fieldClassName="w-[140px]"
        />
        <NumberField
          label="Full spread"
          unit="m"
          min={0.01}
          max={e.maxSpread}
          value={e.distance}
          onChange={e.setDistance}
          fieldClassName="w-[110px]"
        />
      </div>
      <Select<Easing> label="Easing" options={e.easings} value={e.easing} onChange={e.setEasing} />
      <KeyValueList items={e.facts} />
      {!e.keyed ? (
        <p className="text-meta text-fg-mute">
          Not keyed at {e.stopName} yet. Set how far apart to key it.
        </p>
      ) : null}
      <div className="flex gap-2">
        <Button size="sm" icon="trash" disabled={!e.keyed} onClick={e.removeKey}>
          Remove this key
        </Button>
        <Button size="sm" variant="ghost" onClick={e.removeTrack}>
          Remove the track
        </Button>
      </div>
    </section>
  );
}
