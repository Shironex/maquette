import { Button, cn } from '@maquette/ui';
import { sendsNeedYou, sendsSummary, sendsVisible } from '@/state/sends';
import { useSendsButton } from './SendsButton.hooks';

/**
 * Sends: how many sends are under way, once each. Shown while one is, and
 * while a failed send is still listed, so a failure never just vanishes.
 * The count turns clay when one waits on you (a model to make, a send to
 * approve), and a dot says one has failed.
 */
export default function SendsButton() {
  const { counts, open } = useSendsButton();
  if (!sendsVisible(counts)) return null;
  return (
    <Button icon="cube" tooltip={sendsSummary(counts)} onClick={open}>
      Sends
      {counts.inFlight > 0 ? (
        <span
          className={cn(
            'ml-0.5 rounded-pill px-1.5 font-mono text-[11px] leading-[18px]',
            sendsNeedYou(counts) ? 'bg-clay-soft text-clay-text' : 'bg-amber-soft text-amber'
          )}
        >
          {counts.inFlight}
        </span>
      ) : null}
      {counts.failed > 0 ? (
        <>
          <span aria-hidden className="size-1.5 rounded-full bg-danger" />
          <span className="sr-only">, {counts.failed} failed</span>
        </>
      ) : null}
    </Button>
  );
}
