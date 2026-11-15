import { IconButton } from '@maquette/ui';
import { ApprovalCard, useApprovalStack } from '../ApprovalCard';
import { useApprovalTray, useFloatingApprovals } from './ApprovalPrompt.hooks';

/**
 * What an agent asked that waits for a person: a run stopped before Ship
 * ("Ask before shipping"), a send to Tripo or Meshy that would spend
 * credits, a rig doctor fix, a recipe bake or save. Shown beside every view
 * until someone approves or denies it, in a tray docked to the right of the
 * view above the status bar. The view gives up the tray's width while it
 * shows, so no card ever covers a view's rows, numbers or buttons; a sheet
 * opens over the tray, so no card covers a sheet either. Many cards fold to
 * what they ask and scroll inside the tray, the newest at the bottom.
 *
 * The tray folds to a narrow rail with the count, and starts folded in a
 * window too narrow to keep room for a view beside it. Folded cards keep
 * waiting, and the status bar still names them.
 */
export default function ApprovalPrompt() {
  const approvals = useFloatingApprovals();
  const tray = useApprovalTray();
  const stack = useApprovalStack(approvals, !tray.folded);
  if (!approvals?.length) return null;
  const count = approvals.length;
  const waiting = `${count} waiting for approval`;
  if (tray.folded) {
    return (
      <aside
        aria-label="Waiting for approval"
        style={{ width: tray.width }}
        className="flex flex-none flex-col items-center gap-2 border-l border-line bg-app py-3"
      >
        <IconButton
          icon="caret-left"
          label="Show the approval cards"
          hint={waiting}
          tooltipSide="left"
          onClick={tray.unfold}
        />
        <span className="grid min-w-5 place-items-center rounded-full bg-amber-soft px-1.5 text-meta font-semibold text-amber tabular-nums">
          {count}
        </span>
      </aside>
    );
  }
  const cards = approvals.map(a => (
    <ApprovalCard
      key={a.id}
      approval={a}
      floating
      collapsed={stack.collapsed(a.id)}
      onToggle={() => stack.toggle(a.id)}
      armed={stack.armed}
    />
  ));
  return (
    <aside
      aria-label="Waiting for approval"
      style={{ width: tray.width }}
      className="flex min-h-0 flex-none flex-col border-l border-line bg-app"
    >
      <div className="flex flex-none items-center gap-2 px-3 pt-2.5">
        <span className="min-w-0 flex-1 truncate text-meta font-medium text-fg-mute">
          {waiting}
        </span>
        <IconButton
          icon="caret-right"
          size="sm"
          label="Fold the approval cards"
          hint="They keep waiting; the rail shows how many"
          tooltipSide="left"
          onClick={tray.fold}
        />
      </div>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto p-3">
        {/* The cards sit at the bottom; an auto margin keeps the top reachable once they scroll. */}
        <div ref={stack.column} className="mt-auto flex flex-col gap-2.5">
          {cards}
        </div>
      </div>
    </aside>
  );
}
