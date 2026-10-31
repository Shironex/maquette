import { Button } from '@maquette/ui';
import type { PackDecision } from '@/ipc/client';
import type { WarningGroup } from '@/state/packs';

export interface BulkAnswersProps {
  groups: readonly WarningGroup[];
  busy: boolean;
  onDecide: (decisions: readonly PackDecision[]) => void;
}

/** One answer for every model with the same warning, above their cards. */
export default function BulkAnswers({ groups, busy, onDecide }: BulkAnswersProps) {
  if (!groups.length) return null;
  const rows = groups.map(g => {
    const count = g.keep.length;
    return (
      <div
        key={g.kind}
        role="group"
        aria-label={`${g.label}, ${count} models`}
        className="flex flex-col gap-2 rounded-card border border-line bg-raised p-3"
      >
        <span className="text-meta text-fg-dim">
          <b className="font-semibold text-fg">{g.label}</b> · {count} models
        </span>
        <div className="flex flex-wrap gap-2">
          {g.raise.length ? (
            <Button
              size="sm"
              variant="primary"
              disabled={busy}
              tooltip="Decimates each again to the budget its card names, then checks it"
              onClick={() => onDecide(g.raise)}
            >
              Raise all
            </Button>
          ) : null}
          <Button
            size="sm"
            disabled={busy}
            tooltip="Lets each into the pack as the check found it"
            onClick={() => onDecide(g.keep)}
          >
            Keep all
          </Button>
        </div>
      </div>
    );
  });
  return <>{rows}</>;
}
