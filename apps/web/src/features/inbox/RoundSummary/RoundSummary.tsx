import { useRoundSummary } from './RoundSummary.hooks';

function Stat({ label, value, note }: { label: string; value: number; note: string }) {
  return (
    <div className="min-w-0 flex-1 rounded-card border border-line bg-raised px-3 py-2">
      <dt className="text-meta text-fg-mute">{label}</dt>
      <dd className="m-0">
        <span className="text-[18px] leading-6 font-semibold">{value}</span>
        <span className="block truncate text-meta text-fg-mute">{note}</span>
      </dd>
    </div>
  );
}

/** What today's round came to: arrived, accepted, redone, discarded and still waiting. */
export default function RoundSummary() {
  const { round, items, waiting } = useRoundSummary();
  if (!round) return null;
  const arrived = round.accepted + round.discarded + items;
  if (arrived === 0 && round.redoSent === 0 && waiting === 0) return null;
  const credits = Math.round(round.redoCredits ?? 0);
  return (
    <dl aria-label="This round" className="m-0 flex flex-wrap gap-2">
      <Stat label="Arrived" value={arrived} note="today" />
      <Stat label="Accepted" value={round.accepted} note={`${round.toLibrary} to library`} />
      <Stat
        label="Redo sent"
        value={round.redoSent}
        note={credits > 0 ? `${credits} credits` : 'no credits known'}
      />
      <Stat label="Discarded" value={round.discarded} note="kept 30 days" />
      <Stat label="Still waiting" value={waiting} note={waiting === 1 ? 'send' : 'sends'} />
    </dl>
  );
}
