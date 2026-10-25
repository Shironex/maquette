import { ApprovalCard } from '../ApprovalCard';
import { useFloatingApprovals } from './ApprovalPrompt.hooks';

/**
 * What an agent asked that waits for a person: a run stopped before Ship
 * ("Ask before shipping"), or a send to Tripo or Meshy that would spend
 * credits. Shown over every view until someone approves or denies it.
 */
export default function ApprovalPrompt() {
  const approvals = useFloatingApprovals();
  if (!approvals?.length) return null;
  const cards = approvals.map(a => <ApprovalCard key={a.id} approval={a} floating />);
  return (
    <div className="pointer-events-none fixed top-[68px] right-5 z-40 flex w-[400px] flex-col gap-2.5">
      {cards}
    </div>
  );
}
