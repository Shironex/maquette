import type { LabView } from '@/ipc/client';
import { DirectionRow } from './DirectionRow';
import { useEarlierDirections } from './EarlierDirections.hooks';

export interface EarlierDirectionsProps {
  view: LabView;
}

/** The composer's list of earlier directions: pick one to reuse its brief, or move it. */
export default function EarlierDirections({ view }: EarlierDirectionsProps) {
  const { menu, setMenu } = useEarlierDirections();
  const rows = view.directions.map(d => (
    <DirectionRow
      key={d.id}
      view={view}
      direction={d}
      menuOpen={menu === d.id}
      onMenu={open => setMenu(open ? d.id : null)}
    />
  ));
  return (
    <div className="flex flex-col gap-1.5 border-t border-line pt-3.5">
      <span className="mb-1 text-caption text-fg-mute uppercase">Earlier directions</span>
      <ul className="flex list-none flex-col">{rows}</ul>
    </div>
  );
}
