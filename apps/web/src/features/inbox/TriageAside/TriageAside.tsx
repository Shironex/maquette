import { Button, Checkbox, TextField } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { itemTitle } from '@/state/inbox';
import type { AcceptPlan } from '@/state/triage';
import type { AcceptAfter } from '@/state/use-triage-store';
import { DiscardedList } from '../DiscardedList';
import { useLibraryAfter } from './TriageAside.hooks';

export interface TriageAsideProps {
  /** How many models are ticked. */
  count: number;
  plan: AcceptPlan;
  /** The models a redo replaced, which only take room. */
  replaced: readonly InboxItem[];
  working: boolean;
  onAccept: (after: AcceptAfter) => void;
  onDiscard: (ids: readonly string[]) => void;
}

const LABEL = 'text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase';

function Fact({ title, body, value }: { title: string; body: string; value: number }) {
  return (
    <li className="flex gap-3">
      <span className="min-w-0 flex-1">
        <span className="block font-semibold">{title}</span>
        <span className="block text-meta text-fg-mute">{body}</span>
      </span>
      <span className="font-mono text-[13px] text-fg-dim">{value}</span>
    </li>
  );
}

/**
 * The table's side column: what accepting the ticked models does before it
 * runs (versions made, sends closed, the library after), the models a redo
 * replaced with one Discard for all, and the recently discarded ones.
 */
export default function TriageAside({
  count,
  plan,
  replaced,
  working,
  onAccept,
  onDiscard,
}: TriageAsideProps) {
  const lib = useLibraryAfter();
  const n = plan.ids.length;
  const later = plan.later.length;
  const replacedRows = replaced.map(i => (
    <li key={i.id} className="flex items-baseline gap-2">
      <span className="min-w-0 flex-1 truncate">{itemTitle(i)}</span>
      <span className="font-mono text-[11.5px] text-fg-mute">
        v{i.replacedBy} is newer · {formatBytes(i.bytes ?? 0)}
      </span>
    </li>
  ));
  const replacedIds = replaced.map(i => i.id);
  return (
    <aside
      aria-label="What the selection does"
      className="flex w-[340px] flex-none flex-col gap-5 overflow-y-auto border-l border-line bg-panel p-[18px]"
    >
      {count === 0 ? (
        <p className="text-fg-dim">
          Tick models, or a whole group, to accept, redo or discard them together. A model&apos;s
          name opens it one by one with its preview and checks.
        </p>
      ) : (
        <section aria-labelledby="accept-title" className="flex flex-col gap-3.5">
          <div>
            <h2 id="accept-title" className="text-[15px] leading-5 font-semibold">
              Accept {n}
            </h2>
            <p className="text-meta text-fg-mute">One click, no credits</p>
          </div>
          <ul className="flex list-none flex-col gap-3">
            <Fact
              title={`${n} become versions`}
              body={
                later
                  ? `${plan.fresh} as v1 of a new model, ${later} as the next version (${plan.later.slice(0, 3).join(', ')}${later > 3 ? ', more' : ''}).`
                  : `Each as v1 of its model.`
              }
              value={n}
            />
            {plan.handoffs ? (
              <Fact
                title={`${plan.handoffs} By hand ${plan.handoffs === 1 ? 'send closes' : 'sends close'}`}
                body="Their Board cards stop saying Waiting for your model."
                value={plan.handoffs}
              />
            ) : null}
            {plan.skipped ? (
              <Fact
                title={`${plan.skipped} stay here`}
                body="Their files could not be read. Re-export them, then Reprocess."
                value={plan.skipped}
              />
            ) : null}
          </ul>
          <div className="flex flex-col gap-2 rounded-card border border-line bg-raised p-3">
            <Checkbox
              checked={lib.library}
              onChange={lib.setLibrary}
              label="Add to library after"
            />
            {lib.library ? (
              <TextField
                label="Tag all with"
                placeholder="village, props"
                hint="Comma separated. Left empty, each entry keeps its own tags."
                value={lib.tags}
                onChange={e => lib.setTags(e.target.value)}
              />
            ) : (
              <p className="text-meta text-fg-mute">Off unless you pick it.</p>
            )}
          </div>
          <Button
            variant="primary"
            icon="check"
            block
            disabled={n === 0 || working}
            disabledReason={n === 0 ? 'None of these files can be read' : undefined}
            loading={working}
            onClick={() => onAccept(lib.after)}
          >
            {lib.library ? `Accept ${n} and add to library` : `Accept ${n}`}
          </Button>
        </section>
      )}
      {replaced.length ? (
        <section aria-labelledby="replaced-title" className="flex flex-col gap-2.5">
          <div className="flex items-baseline justify-between">
            <h3 id="replaced-title" className={LABEL}>
              Replaced by a redo
            </h3>
            <span className="text-meta text-fg-mute">
              {replaced.length} {replaced.length === 1 ? 'model' : 'models'}
            </span>
          </div>
          <p className="text-meta text-fg-dim">
            A redo kept the old model here. A newer version is accepted, so these only take room.
          </p>
          <ul className="flex list-none flex-col gap-1.5">{replacedRows}</ul>
          <Button
            size="sm"
            variant="danger"
            icon="trash"
            disabled={working}
            onClick={() => onDiscard(replacedIds)}
          >
            Discard {replaced.length} replaced
          </Button>
        </section>
      ) : null}
      {count === 0 ? <DiscardedList /> : null}
      <p className="mt-auto text-meta text-fg-mute">
        Agents can do the same: accept_inbox_models takes a list, and restore_inbox_model puts a
        discarded model back.
      </p>
    </aside>
  );
}
