import { Button, Checkbox, IconButton, NumberField, TextField, Toggle, cn } from '@maquette/ui';
import type { LayoutModel } from '@/ipc/client';
import type { PlainStop } from '@/state/layout';
import { fixed } from '../fixed';
import { Vec3Fields } from '../Vec3Fields';
import { useStopDetails } from './StopDetails.hooks';

export interface StopDetailsProps {
  stop: PlainStop;
  index: number;
  count: number;
  models: LayoutModel[];
}

/** The selected camera stop's name, order, camera fields and first view models. */
export default function StopDetails({ stop, index, count, models }: StopDetailsProps) {
  const { store, looking } = useStopDetails(stop);
  const checks =
    stop.models !== null
      ? models.map(m => (
          <Checkbox
            key={m.asset}
            label={m.name}
            checked={stop.models?.includes(m.asset) ?? false}
            onChange={on =>
              store.setStop(stop.id, {
                models: on
                  ? [...(stop.models ?? []), m.asset]
                  : (stop.models ?? []).filter(id => id !== m.asset),
              })
            }
          />
        ))
      : null;
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-end gap-2">
        <TextField
          label={index === 0 ? 'Name · the view the page opens on' : 'Name'}
          value={stop.name}
          maxLength={60}
          fieldClassName="flex-1"
          onChange={e => store.setStop(stop.id, { name: e.target.value })}
        />
        <IconButton
          icon="arrow-up"
          label="Earlier in the tour"
          variant="secondary"
          disabled={index === 0}
          onClick={() => store.moveStop(stop.id, -1)}
        />
        <IconButton
          icon="arrow-down"
          label="Later in the tour"
          variant="secondary"
          disabled={index === count - 1}
          onClick={() => store.moveStop(stop.id, 1)}
        />
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          icon="video-camera"
          disabled={!store.pose}
          onClick={() => store.setStopFromView(stop.id)}
          title="Puts the stop where the view's camera stands now"
        >
          Set from view
        </Button>
        <Button
          size="sm"
          icon="eye"
          aria-pressed={looking}
          className={cn(looking && 'border-clay-line text-clay-text')}
          onClick={() => store.lookThrough(looking ? null : stop.id)}
        >
          {looking ? 'Stop looking' : 'Look through'}
        </Button>
        <IconButton
          icon="trash"
          label={`Delete ${stop.name || 'this stop'}`}
          size="sm"
          variant="secondary"
          className="ml-auto"
          onClick={() => store.removeStop(stop.id)}
        />
      </div>
      <Vec3Fields
        label="Camera"
        unit="m"
        digits={3}
        value={stop.position}
        onChange={position => store.setStop(stop.id, { position })}
      />
      <Vec3Fields
        label="Looks at"
        unit="m"
        digits={3}
        value={stop.target}
        onChange={target => store.setStop(stop.id, { target })}
      />
      <NumberField
        label="Field of view"
        size="sm"
        unit="degrees"
        min={10}
        max={120}
        value={stop.fov}
        format={fixed(1)}
        onChange={fov => store.setStop(stop.id, { fov })}
        hint="Vertical, at 16:9 like the posters."
      />
      {index === 0 ? (
        <div className="flex flex-col gap-2">
          <Toggle
            label="Count what the camera sees"
            description="The first view budget counts the models in this stop's frame. Turn it off to pick them yourself."
            checked={stop.models === null}
            onChange={on =>
              store.setStop(stop.id, {
                models: on ? null : [...(store.view?.firstView.models.map(m => m.asset) ?? [])],
              })
            }
          />
          {stop.models !== null ? <div className="flex flex-col gap-1.5 pl-1">{checks}</div> : null}
        </div>
      ) : null}
    </div>
  );
}
