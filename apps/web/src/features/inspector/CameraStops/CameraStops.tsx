import { Icon, cn } from '@maquette/ui';
import type { InspectView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { stopsHelp, type Stop } from '@/state/inspector';
import { badge } from '../badge';
import { SectionTitle } from '../SectionTitle';
import { useCameraStops } from './CameraStops.hooks';

export interface CameraStopsProps {
  view: InspectView;
  stops: Stop[];
  /** Live renders of the stops without a poster, by stop index. */
  shots: Record<number, string>;
}

/** The scene's camera stops as posters to click, with the current one ringed. */
export default function CameraStops({ view, stops, shots }: CameraStopsProps) {
  const { stop, flyTo } = useCameraStops();
  const columns = `repeat(${Math.max(5, stops.length)}, minmax(0, 1fr))`;
  const cards = stops.map((s, i) => {
    const src = s.poster ? fileUrl(s.poster) : (shots[i] ?? null);
    const on = stop?.index === i;
    return (
      <button
        key={`${s.name}-${i}`}
        type="button"
        aria-pressed={on}
        aria-label={`Fly to ${s.name}`}
        onClick={() => flyTo(i)}
        className={cn(
          'relative h-[86px] cursor-pointer overflow-hidden rounded-[8px] bg-studio',
          on
            ? 'shadow-[0_0_0_2px_var(--color-clay)]'
            : 'shadow-[0_0_0_1px_var(--color-line)] hover:shadow-[0_0_0_1px_var(--color-line-strong)]'
        )}
      >
        {src ? (
          <img src={src} alt="" className="block size-full object-cover" />
        ) : (
          <span className="grid size-full place-items-center text-fg-mute">
            <Icon name="camera" />
          </span>
        )}
        <span className={cn(badge, 'bottom-1.5 left-1.5 h-5')}>{s.name}</span>
      </button>
    );
  });
  return (
    <div className="flex flex-none flex-col gap-2">
      <SectionTitle aside={stopsHelp(view)}>Camera stops</SectionTitle>
      <div className="grid gap-2.5" style={{ gridTemplateColumns: columns }}>
        {cards}
      </div>
    </div>
  );
}
