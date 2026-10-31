import { Icon, cn } from '@maquette/ui';
import { usePageHotspots } from './PageHotspots.hooks';

/**
 * Pins on the page where the camera sees a prop with click or hover triggers.
 * They mark the page and never take its pointer, so the page still answers.
 */
export default function PageHotspots() {
  const { spots, line } = usePageHotspots();
  const pins = spots.map(s => (
    <li
      key={s.key}
      className="absolute inline-flex h-6 -translate-x-1/2 -translate-y-1/2 items-center gap-1.5 rounded-pill border border-focus bg-[rgba(28,27,25,0.9)] px-2 text-[11.5px] font-medium whitespace-nowrap text-fg"
      style={{ left: `${s.x * 100}%`, top: `${s.y * 100}%` }}
    >
      <Icon
        name={s.kind === 'click' ? 'cursor-click' : 'hand-pointing'}
        size={12}
        className={cn('flex-none', s.kind === 'click' ? 'text-clay-text' : 'text-fg-dim')}
      />
      {s.label}
    </li>
  ));
  return (
    <div className="pointer-events-none absolute inset-0">
      <ul aria-label="Hotspots" className="m-0 list-none p-0">
        {pins}
      </ul>
      {line ? (
        <span className="absolute top-3 left-1/2 -translate-x-1/2 rounded-control bg-[rgba(28,27,25,0.9)] px-2 py-1 text-[11.5px] whitespace-nowrap text-fg-dim">
          {line}
        </span>
      ) : null}
    </div>
  );
}
