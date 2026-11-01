export interface StatTileProps {
  label: string;
  value: string;
  note: string;
  /** A text colour class for the value, for example `text-sage`. */
  tone?: string;
}

/** One figure in a library's stats row: a label, a big value and a small note. */
export default function StatTile({ label, value, note, tone }: StatTileProps) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1 rounded-card border border-line bg-raised px-3 py-3 min-[1280px]:px-4">
      <span className="truncate text-meta text-fg-mute" title={label}>
        {label}
      </span>
      <span className={tone ?? 'text-fg'}>
        <span className="text-[22px] leading-7 font-semibold whitespace-nowrap tracking-[-0.01em]">
          {value}
        </span>
        <small className="block text-meta text-fg-mute">{note}</small>
      </span>
    </div>
  );
}
