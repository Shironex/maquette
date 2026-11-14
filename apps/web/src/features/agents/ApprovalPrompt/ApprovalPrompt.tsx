import { ApprovalCard, useApprovalStack } from '../ApprovalCard';
import { APPROVAL_TRAY_WIDTH, useFloatingApprovals } from './ApprovalPrompt.hooks';

/**
 * What an agent asked that waits for a person: a run stopped before Ship
 * ("Ask before shipping"), a send to Tripo or Meshy that would spend
 * credits, a rig doctor fix, a recipe bake or save. Shown beside every view
 * until someone approves or denies it, in a tray docked to the right of the
 * view above the status bar. The view gives up the tray's width while it
 * shows, so no card ever covers a view's rows, numbers or buttons; a sheet
 * opens over the tray, so no card covers a sheet either. Many cards fold to
 * what they ask and scroll inside the tray, the newest at the bottom.
 */
export default function ApprovalPrompt() {
  const approvals = useFloatingApprovals();
  const stack = useApprovalStack(approvals);
  if (!approvals?.length) return null;
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
      style={{ width: APPROVAL_TRAY_WIDTH }}
      className="flex min-h-0 flex-none flex-col overflow-y-auto border-l border-line bg-app p-3"
    >
      {/* The cards sit at the bottom; an auto margin keeps the top reachable once they scroll. */}
      <div ref={stack.column} className="mt-auto flex flex-col gap-2.5">
        {cards}
      </div>
    </aside>
  );
}
