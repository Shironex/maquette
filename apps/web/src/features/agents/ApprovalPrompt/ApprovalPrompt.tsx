import { ApprovalCard } from '../ApprovalCard';
import { useFloatingApprovals } from './ApprovalPrompt.hooks';

/**
 * What an agent asked that waits for a person: a run stopped before Ship
 * ("Ask before shipping"), or a send to Tripo or Meshy that would spend
 * credits. Shown over every view until someone approves or denies it, in the
 * bottom right corner above the status bar: the top right is where the views'
 * header controls are, and a prompt there blocked their clicks. Many
 * prompts scroll inside the column and never reach the header row.
 */
export default function ApprovalPrompt() {
  const approvals = useFloatingApprovals();
  if (!approvals?.length) return null;
  const cards = approvals.map(a => <ApprovalCard key={a.id} approval={a} floating />);
  return (
    <div className="pointer-events-none fixed right-5 bottom-11 z-40 flex max-h-[calc(100vh-240px)] w-[400px] flex-col gap-2.5 overflow-y-auto">
      {cards}
    </div>
  );
}
