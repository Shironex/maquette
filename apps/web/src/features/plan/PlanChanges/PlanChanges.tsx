import { Button, cn, Icon, IconButton, SegmentedControl, type IconName } from '@maquette/ui';
import type { ChangeMark } from '@/ipc/client';
import type { LogFilter } from '@/state/plan-log';
import { usePlanChanges } from './PlanChanges.hooks';

const MARK_ICON: Record<ChangeMark, IconName> = {
  add: 'plus',
  edit: 'note-pencil',
  move: 'arrows-out-cardinal',
  remove: 'trash',
  link: 'link-simple',
  mixed: 'stack',
  undo: 'arrow-counter-clockwise',
  rename: 'tag',
  create: 'sparkle',
  keep: 'check',
};

const KEEP_TIP =
  'Accept this agent change: the log stops holding your earlier version for it, which frees room. Earlier agent changes to the same part of the node are kept with it. It stays undoable while it is among the newest changes.';

const FILTERS: { value: LogFilter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'agents', label: 'Agents' },
  { value: 'mine', label: 'Mine' },
];

const count = (n: number) => `${n} change${n === 1 ? '' : 's'}`;

/**
 * The Changes panel beside the canvas: every write to it, newest first, in
 * runs (an agent's calls together, a person's changes close in time), each
 * run and change with its undo. The footer says the rules agents follow.
 */
export default function PlanChanges() {
  const v = usePlanChanges();
  const runs = v.runs.map(run => {
    const open = v.isOpen(run);
    const agent = run.by !== null;
    const newest = run.lines[0];
    const undoRun = agent && open && run.undoable.length > 1;
    const keepRun = agent && open && run.keepable.length > 1;
    const rows = open
      ? run.lines.map(line => (
          <div key={line.id} className={cn('plan-change', line.undone && 'is-undone')}>
            <span className={cn('plan-change-mark', agent ? 'is-agent' : 'is-me')}>
              <Icon name={MARK_ICON[line.mark]} size={12} />
            </span>
            <span className="min-w-0 break-words">
              {line.summary}
              {line.undone ? <span className="sr-only"> (undone)</span> : null}
            </span>
            <span className="flex items-center gap-0.5">
              {line.keepable ? (
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={v.busy}
                  aria-label={`Keep: ${line.summary}`}
                  tooltip={KEEP_TIP}
                  onClick={() => v.keep([line.id])}
                >
                  Keep
                </Button>
              ) : null}
              {line.undoable ? (
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={v.busy}
                  aria-label={`Undo: ${line.summary}`}
                  onClick={() => v.undo([line.id])}
                >
                  Undo
                </Button>
              ) : (
                <span className="text-[11px] text-fg-mute">
                  {line.undone ? 'undone' : line.kept ? 'kept' : ''}
                </span>
              )}
            </span>
          </div>
        ))
      : null;
    return (
      <div
        role="group"
        key={run.key}
        aria-label={`${run.by ?? 'You'}, ${count(run.lines.length)}`}
        className={cn('plan-run', agent && 'is-agent')}
      >
        <div className="plan-run-head">
          <span className={cn('plan-run-avatar', !agent && 'is-me')} aria-hidden="true">
            <Icon name={agent ? 'robot' : 'person-simple'} size={15} />
          </span>
          <div className="min-w-0">
            <h3 className="truncate text-[13px] leading-[17px] font-semibold text-fg">
              {run.by ?? 'You'}
            </h3>
            <p className="text-[11.5px] leading-[15px] text-fg-mute">
              {newest ? v.when(newest.at) : ''} · {count(run.lines.length)}
            </p>
          </div>
          <span className="ml-auto" />
          <IconButton
            icon="caret-down"
            size="sm"
            variant="ghost"
            label={open ? 'Fold the changes' : 'Show the changes'}
            aria-expanded={open}
            className={cn('transition-transform', !open && '-rotate-90')}
            onClick={() => v.toggle(run)}
          />
        </div>
        {keepRun || undoRun ? (
          <div className="plan-run-actions">
            {keepRun ? (
              <Button
                size="sm"
                disabled={v.busy}
                tooltip="Accepts every agent change of this run to your content, freeing the room the log holds to undo them"
                onClick={() => v.keep(run.keepable)}
              >
                Keep run
              </Button>
            ) : null}
            {undoRun ? (
              <Button
                size="sm"
                disabled={v.busy}
                tooltip="Undoes every change of this run that is not undone yet"
                onClick={() => v.undo(run.undoable)}
              >
                Undo run
              </Button>
            ) : null}
          </div>
        ) : null}
        {rows}
      </div>
    );
  });
  return (
    <aside aria-label="Changes" className="plan-side">
      <div className="plan-side-head">
        <h2 className="flex-1 text-[15px] leading-5 font-semibold text-fg">Changes</h2>
        <SegmentedControl label="Show" options={FILTERS} value={v.filter} onChange={v.setFilter} />
        <IconButton icon="x" size="sm" variant="ghost" label="Close Changes" onClick={v.close} />
      </div>
      <div className="plan-side-body">
        {v.error ? (
          <p role="alert" className="text-meta text-danger">
            The change log could not be read. {v.error}
          </p>
        ) : runs.length ? (
          runs
        ) : (
          <p className="text-meta text-fg-mute">
            {v.filter === 'all'
              ? "No changes yet. Each edit, yours or an agent's, shows up here."
              : v.filter === 'agents'
                ? 'No agent has changed this canvas yet.'
                : 'You have not changed this canvas yet.'}
          </p>
        )}
      </div>
      <p className="plan-side-foot">
        Agents never remove your nodes or links, or nodes you wrote in. What an agent changed on
        your content stays undoable until you undo it or press Keep, also after a restart. Other
        changes stay undoable while they are among the newest in the log (at least the last 512 KB);
        one edit over 256 KB keeps only its summary.
      </p>
    </aside>
  );
}
