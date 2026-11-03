import type { PlanPreset } from '@/ipc/client';

/** A small picture of the shape a preset starts with, drawn in the theme's colours. */
export default function PresetArt({ preset }: { preset: PlanPreset }) {
  // Game mechanics: three system cards, each with its name line and three rows.
  const columns = [4, 20, 36].map(x => {
    const rows = [9, 15, 21, 27].map(y => (
      <rect
        key={y}
        x={x + 2}
        y={y}
        width="8"
        height="3"
        rx="1.5"
        className={y === 9 ? 'fill-fg-mute' : 'fill-line-strong'}
      />
    ));
    return (
      <g key={x}>
        <rect x={x} y="6" width="12" height="28" rx="2" className="fill-line" />
        {rows}
      </g>
    );
  });
  return (
    <svg viewBox="0 0 52 40" aria-hidden="true" className="block size-full">
      {preset === 'brainstorm' ? (
        <>
          <path
            d="M14 10 L26 20 M14 30 L26 20 M38 20 L46 12 M38 20 L46 28"
            className="stroke-fg-mute"
          />
          <rect x="6" y="6" width="10" height="7" rx="2" className="fill-line-strong" />
          <rect x="6" y="27" width="10" height="7" rx="2" className="fill-line-strong" />
          <rect x="24" y="15" width="14" height="10" rx="2" className="fill-fg-mute" />
          <rect x="44" y="8" width="8" height="7" rx="2" className="fill-line-strong" />
          <rect x="44" y="25" width="8" height="7" rx="2" className="fill-line-strong" />
        </>
      ) : preset === 'story' ? (
        <>
          <path d="M0 14 H52 M0 27 H52" className="stroke-line" strokeDasharray="2 2" />
          <rect x="8" y="4" width="10" height="6" rx="2" className="fill-line-strong" />
          <rect x="30" y="4" width="10" height="6" rx="2" className="fill-line-strong" />
          <path d="M15 20 H20 M30 20 H34" className="stroke-fg-dim" />
          <rect x="4" y="17" width="11" height="6" rx="2" className="fill-line-strong" />
          <rect x="20" y="17" width="10" height="6" rx="2" className="fill-line-strong" />
          <rect x="34" y="17" width="12" height="6" rx="2" className="fill-line-strong" />
          <rect x="14" y="30" width="10" height="6" rx="2" className="fill-line-strong" />
          <rect x="32" y="30" width="10" height="6" rx="2" className="fill-line-strong" />
        </>
      ) : (
        columns
      )}
    </svg>
  );
}
