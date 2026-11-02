import { Button, Icon, IconButton } from '@maquette/ui';
import { useAgentToast } from './AgentToast.hooks';

/** An agent changed the open canvas: who, what, and undo for the whole run. */
export default function AgentToast() {
  const v = useAgentToast();
  if (!v.toast) return null;
  const more = v.changes > 1 ? `${v.changes} changes in this run` : v.toast.summary;
  return (
    <div role="status" className="plan-toast">
      <Icon name="robot" size={16} className="shrink-0 text-[var(--plan-agent)]" />
      <span className="min-w-0">
        <span className="block truncate text-fg">{v.toast.by} changed the canvas</span>
        <span className="block truncate text-[11.5px] text-fg-mute">{more}</span>
      </span>
      <Button size="sm" variant="ghost" onClick={v.show}>
        Show changes
      </Button>
      <Button
        size="sm"
        icon="arrow-counter-clockwise"
        disabled={v.busy}
        tooltip="Undoes every change of this run; what you changed since stays"
        onClick={v.undoRun}
      >
        Undo run
      </Button>
      <IconButton icon="x" size="sm" variant="ghost" label="Dismiss" onClick={v.dismiss} />
    </div>
  );
}
