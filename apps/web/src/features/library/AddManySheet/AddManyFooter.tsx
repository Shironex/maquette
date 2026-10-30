import { Button, ProgressBar } from '@maquette/ui';
import type { AddManyProgress, AddManyResult } from '@/state/use-add-many-store';

export interface AddManyFooterProps {
  progress: AddManyProgress | null;
  result: AddManyResult | null;
  /** How many models Add copies. */
  count: number;
  ready: boolean;
  onStart: () => void;
  onStop: () => void;
  onClose: () => void;
}

/** The sheet's foot: Add, the progress with Stop, or what was done. */
export default function AddManyFooter(p: AddManyFooterProps) {
  const models = (n: number) => `${n} ${n === 1 ? 'model' : 'models'}`;
  let body;
  if (p.progress) {
    const { done, total, current, stopping } = p.progress;
    body = (
      <>
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <div className="flex items-baseline gap-2">
            <b className="truncate">{current ? `Adding ${current}` : 'Starting'}</b>
            <span className="flex-1" />
            <span className="font-mono text-meta text-fg-dim">
              {done} / {total}
            </span>
          </div>
          <ProgressBar value={total ? (done / total) * 100 : 0} label="Adding to the library" />
          <span className="text-meta text-fg-mute">
            {stopping ? 'Stopping after this one' : 'Copies only, no credits'}
          </span>
        </div>
        <Button
          disabled={stopping}
          onClick={p.onStop}
          tooltip="Finishes the model being copied, then stops"
        >
          Stop after this one
        </Button>
      </>
    );
  } else if (p.result) {
    const r = p.result;
    const parts = [
      `Added ${models(r.added)}`,
      r.skipped ? `${r.skipped} left out` : null,
      r.failed.length ? `${r.failed.length} failed` : null,
    ];
    const said = parts.filter(Boolean).join(', ');
    const why = r.failed.map(f => `${f.name}: ${f.why}`).join(' ');
    body = (
      <>
        <div className="flex min-w-0 flex-1 flex-col">
          <b>
            {said}
            {r.stopped ? ', stopped' : ''}
          </b>
          {r.failed.length ? (
            <span className="truncate text-meta text-danger">{why}</span>
          ) : (
            <span className="text-meta text-fg-mute">
              Tags and the collection are set as picked.
            </span>
          )}
        </div>
        <Button variant="primary" onClick={p.onClose}>
          Done
        </Button>
      </>
    );
  } else {
    body = (
      <>
        <span className="flex-1 text-meta text-fg-mute">Copies only, no credits.</span>
        <Button onClick={p.onClose}>Cancel</Button>
        <Button
          variant="primary"
          icon="plus"
          disabled={!p.ready || p.count === 0}
          disabledReason={p.ready ? 'Nothing to add' : 'Pick a project first'}
          onClick={p.onStart}
        >
          Add {models(p.count)}
        </Button>
      </>
    );
  }
  return <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">{body}</div>;
}
