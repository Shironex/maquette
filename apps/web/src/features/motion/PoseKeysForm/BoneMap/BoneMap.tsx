import { Select, cn } from '@maquette/ui';
import { counted } from '@/lib/format';
import { boneWords } from '@/state/recipe-bones';
import { ALL_BONES, MAP_LINES, MAP_SPOTS } from './BoneMap.hooks';

export interface BoneMapProps {
  /** The bone picked. */
  bone: string;
  /** How many keys turn each bone. */
  counts: ReadonlyMap<string, number>;
  onPick: (bone: string) => void;
}

const spot = new Map(MAP_SPOTS.map(s => [s.bone, s]));

/**
 * The bone picker: a body map (its right on your left, R and L marked) with
 * a dot per body bone, filled when it has keys, and a list of the keyed
 * bones with their counts. Any bone, fingers too, picks from the list below.
 */
export default function BoneMap({ bone, counts, onPick }: BoneMapProps) {
  const lines = MAP_LINES.map(([a, b]) => {
    const p = spot.get(a)!;
    const q = spot.get(b)!;
    return <line key={`${a}-${b}`} x1={p.x} y1={p.y} x2={q.x} y2={q.y} />;
  });
  const dots = MAP_SPOTS.map(s => {
    const keys = counts.get(s.bone) ?? 0;
    const picked = s.bone === bone;
    return (
      <button
        key={s.bone}
        type="button"
        aria-label={`${boneWords(s.bone)}${keys ? `, ${counted(keys, 'key')}` : ''}`}
        aria-pressed={picked}
        title={boneWords(s.bone)}
        onClick={() => onPick(s.bone)}
        className={cn(
          'absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 focus-visible:outline-2 focus-visible:outline-focus',
          keys ? 'border-clay bg-clay' : 'border-fg-mute bg-panel',
          picked && 'size-3.5 border-fg ring-2 ring-clay/50'
        )}
        style={{ left: `${s.x}%`, top: `${s.y}%` }}
      />
    );
  });
  const listed = [...new Set([...counts.keys(), bone])].sort();
  const rows = listed.map(b => (
    <li key={b}>
      <button
        type="button"
        aria-pressed={b === bone}
        onClick={() => onPick(b)}
        className={cn(
          'flex w-full items-center justify-between gap-2 rounded-control px-2 py-1 text-left text-meta hover:bg-hover',
          b === bone ? 'bg-hover text-fg' : 'text-fg-dim'
        )}
      >
        <span className="flex items-center gap-1.5">
          <i
            aria-hidden
            className={cn(
              'size-2 rounded-full border',
              counts.get(b) ? 'border-clay bg-clay' : 'border-fg-mute'
            )}
          />
          {boneWords(b)}
        </span>
        <span className="text-fg-mute">
          {counts.get(b) ? counted(counts.get(b)!, 'key') : 'no keys'}
        </span>
      </button>
    </li>
  ));
  return (
    <div role="group" aria-label="Bones" className="flex gap-3">
      <div className="relative h-[200px] w-[110px] flex-none rounded-card bg-raised">
        <span className="absolute top-1 left-1.5 text-[10px] font-semibold text-fg-mute">R</span>
        <span className="absolute top-1 right-1.5 text-[10px] font-semibold text-fg-mute">L</span>
        <svg
          aria-hidden
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          className="absolute inset-0 size-full stroke-line-strong"
          strokeWidth={1.2}
          vectorEffect="non-scaling-stroke"
        >
          {lines}
        </svg>
        {dots}
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <ul className="m-0 flex list-none flex-col gap-0.5 p-0">{rows}</ul>
        <Select label="Any bone" size="sm" options={ALL_BONES} value={bone} onChange={onPick} />
        <p className="text-meta text-fg-mute">
          Click a bone in the view or here. Filled dots have keys.
        </p>
      </div>
    </div>
  );
}
