import { FRAME_TARGET_MS, average, graphPoints, targetY } from '@/state/inspector';

const W = 560;
const H = 44;

/** The frame time card: the last second's average and 30 seconds of samples. */
export function FrameTime({ samples }: { samples: readonly number[] }) {
  const recent = average(samples.slice(-4));
  const line = targetY(samples, H);
  const over = recent > FRAME_TARGET_MS;
  const label =
    samples.length === 0
      ? 'Frame time, measuring'
      : `Frame time over the last ${Math.round(samples.length / 4)} seconds, now ${recent.toFixed(1)} ms, ${over ? 'over' : 'under'} the 16.7 ms target`;
  return (
    <div className="flex flex-none items-center gap-5 rounded-card border border-line bg-raised px-4 py-3">
      <div className="flex w-[120px] flex-none flex-col gap-0.5">
        <span className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
          Frame time
        </span>
        <span
          className={`font-mono text-[22px] leading-7 tabular-nums ${over ? 'text-danger' : 'text-fg'}`}
        >
          {samples.length ? recent.toFixed(1) : '…'}{' '}
          <span className="text-[13px] text-fg-mute">ms</span>
        </span>
      </div>
      <svg
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        role="img"
        aria-label={label}
        className="h-11 min-w-0 flex-1"
      >
        <line
          x1="0"
          x2={W}
          y1={line}
          y2={line}
          stroke="var(--color-line-strong)"
          strokeWidth="1"
          strokeDasharray="3 4"
          vectorEffect="non-scaling-stroke"
        />
        <polyline
          fill="none"
          stroke={over ? 'var(--color-danger)' : 'var(--color-sage)'}
          strokeWidth="1.5"
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
          points={graphPoints(samples, W, H)}
        />
      </svg>
      <div className="flex flex-none flex-col items-end gap-0.5">
        <span className="text-meta text-fg-mute">60 fps target</span>
        <span className="font-mono text-[12px] text-fg-dim tabular-nums">16.7 ms</span>
      </div>
    </div>
  );
}
