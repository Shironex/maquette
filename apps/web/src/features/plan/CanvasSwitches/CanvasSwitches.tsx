import { Button, cn } from '@maquette/ui';
import { useCanvasSwitches } from './CanvasSwitches.hooks';

/** Auto layout after agents, and the Overlaps panel on a Game mechanics canvas. */
export default function CanvasSwitches() {
  const v = useCanvasSwitches();
  return (
    <>
      {v.overlaps ? (
        <Button
          size="sm"
          icon="warning"
          aria-pressed={v.overlaps.open}
          tooltip="System cards that share a verb, a look or a key"
          className={cn(v.overlaps.open && 'border-clay-line text-clay-text')}
          onClick={v.overlaps.toggle}
        >
          Overlaps
          {v.overlaps.flagged ? (
            <>
              <span className="sr-only">, </span>
              <span className="plan-count">{v.overlaps.flagged}</span>
              <span className="sr-only"> found</span>
            </>
          ) : null}
        </Button>
      ) : null}
      <Button
        size="sm"
        icon="magic-wand"
        aria-pressed={v.auto}
        disabled={v.busy}
        disabledReason="A change is on its way"
        tooltip={
          v.auto
            ? "On: an agent's nodes that land on others move clear after it writes. Your nodes never move."
            : 'Off: agents place their nodes where they say and never tidy them. Your nodes never move.'
        }
        className={cn(v.auto && 'border-clay-line text-clay-text')}
        onClick={v.flipAuto}
      >
        Auto layout<span className="sr-only"> after agents</span>
      </Button>
    </>
  );
}
