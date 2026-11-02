import { Button, IconButton } from '@maquette/ui';
import { useNextStepStrip } from './NextStepStrip.hooks';

/** One sentence and one button: what the open project is waiting on. */
export default function NextStepStrip() {
  const next = useNextStepStrip();
  if (!next) return null;
  const { step, act, dismiss } = next;
  return (
    <section
      aria-label="Next step"
      className="flex flex-none items-center gap-3 rounded-card border border-line bg-panel py-1.5 pr-1.5 pl-3.5"
    >
      <span className="flex-none text-meta font-semibold text-clay-text">Next</span>
      <p className="m-0 min-w-0 flex-1 truncate text-fg-dim" title={step.sentence}>
        {step.sentence}
      </p>
      <Button size="sm" onClick={act}>
        {step.button}
      </Button>
      <IconButton
        icon="x"
        size="sm"
        variant="ghost"
        label="Dismiss this step"
        hint="Stays away in this project until this step changes"
        onClick={dismiss}
      />
    </section>
  );
}
