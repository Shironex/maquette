import type { ReactNode } from 'react';
import { IconButton, cn } from '@maquette/ui';
import { boneWords } from '@/state/recipe-bones';
import { KIND_WORDS } from '@/state/recipe-draft';
import { keyedBones } from '@/state/recipe-keys';
import { STATUS_WORDS, across, at, frameReadout, type LaneBlock } from '@/state/recipe-lanes';
import { pinLabel } from '@/state/recipe-pins';
import { ChangeBar } from './ChangeBar';
import { PoseKeyLane } from './PoseKeyLane';
import { useRecipeLanes } from './RecipeLanes.hooks';

export interface RecipeLanesProps {
  /** The frame on show, 0 to `frames`. */
  frame: number;
  playing: boolean;
  onToggle: () => void;
  onScrub: (frame: number) => void;
}

/** The label column's width; the playhead line sits past it. */
const LABEL = 104;

/** A bone's lane label, short: "R upper arm". */
const laneWords = (bone: string) =>
  boneWords(bone)
    .replace(/^Right /, 'R ')
    .replace(/^Left /, 'L ');

const STATUS_LOOK = {
  found: 'border-line',
  'hash-differs': 'border-amber',
  missing: 'border-danger border-dashed',
  rest: 'border-line',
} as const;

function Lane({
  label,
  children,
  group,
}: {
  label: string;
  children?: ReactNode;
  group?: boolean;
}) {
  if (group)
    return (
      <>
        <div className="pt-2 pb-0.5 text-[11px] font-semibold tracking-wide text-fg-mute uppercase">
          {label}
        </div>
        <div />
      </>
    );
  return (
    <>
      <div className="flex h-7 items-center truncate text-meta text-fg-dim">{label}</div>
      <div data-lane className="relative h-7 border-t border-line/60">
        {children}
      </div>
    </>
  );
}

/** A block of a base edit on the ruler, which opens its settings. */
function Block({
  block,
  frames,
  picked,
  onPick,
  className,
  name,
}: {
  block: LaneBlock;
  frames: number;
  picked: boolean;
  onPick: () => void;
  className: string;
  name: string;
}) {
  return (
    <button
      type="button"
      aria-label={name}
      aria-pressed={picked}
      title={block.label}
      onClick={onPick}
      className={cn(
        'absolute inset-y-1 min-w-1.5 overflow-hidden rounded-[5px] px-2 text-left text-[11px] font-medium whitespace-nowrap focus-visible:outline-2 focus-visible:outline-focus',
        className,
        picked && 'outline-2 outline-fg'
      )}
      style={{ left: at(block.first, frames), width: across(block.first, block.last, frames) }}
    >
      {block.label}
    </button>
  );
}

/**
 * The recipe on one frame ruler over the bake's length: the base edits
 * (clips with their blend windows, trims, retimes, holds, the mirror) above
 * a bar per change, with the playhead, play and the frame on show.
 */
export default function RecipeLanes({ frame, playing, onToggle, onScrub }: RecipeLanesProps) {
  const v = useRecipeLanes();
  if (!v.dt || !v.lanes) return null;
  const { timeline: t, toFrame, toShare } = v.dt;
  const { frames, fps } = t;
  const lanes = v.lanes;
  const share = frames > 0 ? frame / frames : 0;
  const marks = lanes.marks.map(m => (
    <span key={m.frame} className="absolute top-0 h-full" style={{ left: at(m.frame, frames) }}>
      <i className={cn('absolute top-0 w-px bg-line-strong', m.major ? 'h-2' : 'h-1')} />
      {m.major ? (
        <span className="absolute top-2 -translate-x-1/2 font-mono text-[10px] text-fg-mute">
          {m.frame}
        </span>
      ) : null}
    </span>
  ));
  const clipBlocks = lanes.clips.map((b, i) => (
    <Block
      key={b.key}
      block={b}
      frames={frames}
      name={`Clip ${i + 1}, ${b.label}, ${STATUS_WORDS[b.status]}`}
      picked={v.isPicked('clips')}
      onPick={() => v.toggle('clips')}
      className={cn('border bg-raised text-fg', STATUS_LOOK[b.status])}
    />
  ));
  const blends = lanes.blends.map(b => (
    <span
      key={b.key}
      title={b.label}
      className="pointer-events-none absolute inset-y-0.5 rounded-[4px] bg-clay/35 ring-1 ring-clay/60"
      style={{ left: at(b.first, frames), width: across(b.first, b.last, frames) }}
    />
  ));
  const trims = lanes.trims.map(b => (
    <Block
      key={b.key}
      block={b}
      frames={frames}
      name={`Trim of clip ${Number(b.key.slice(5)) + 1}, ${b.label.toLowerCase()}`}
      picked={v.isPicked('trim', b.key.slice(5))}
      onPick={() => v.toggle('trim', b.key.slice(5))}
      className="bg-transparent text-center text-fg-dim hover:bg-hover"
    />
  ));
  const timeBlock = (b: (typeof lanes.retimes)[number], look: string) => (
    <Block
      key={b.key}
      block={b}
      frames={frames}
      name={b.label}
      picked={v.isPicked('time', String(b.index))}
      onPick={() => v.toggle('time', String(b.index))}
      className={look}
    />
  );
  const retimes = lanes.retimes.map(b => timeBlock(b, 'bg-sage/35 text-fg'));
  const holds = lanes.holds.map(b => timeBlock(b, 'bg-amber/30 text-fg'));
  const poseLanes = (layer: (typeof v.layers)[number]) => {
    const picked = v.isPicked('layer', layer.id);
    const bones = picked ? keyedBones(layer) : [];
    return [
      <Lane key={layer.id} label={KIND_WORDS.pose}>
        <button
          type="button"
          aria-label={`Pose keys ${layer.id}`}
          aria-pressed={picked}
          onClick={() => v.toggle('layer', layer.id)}
          className={cn(
            'absolute inset-y-1 inset-x-0 rounded-[5px] bg-clay/15 focus-visible:outline-2 focus-visible:outline-focus',
            picked && 'outline-2 outline-fg'
          )}
        />
        <PoseKeyLane id={layer.id} bone={null} frames={frames} />
      </Lane>,
      ...bones.map(bone => (
        <Lane key={`${layer.id}-${bone}`} label={laneWords(bone)}>
          <PoseKeyLane id={layer.id} bone={bone} frames={frames} />
        </Lane>
      )),
    ];
  };
  const changes = v.layers.flatMap(layer =>
    layer.kind === 'pose'
      ? poseLanes(layer)
      : [
          <Lane key={layer.id} label={KIND_WORDS[layer.kind]}>
            <ChangeBar
              layer={layer}
              label={layer.kind === 'grip' ? v.gripLabel(layer) : undefined}
              look={layer.kind === 'grip' ? 'bg-clay/45 text-fg' : undefined}
              frames={frames}
              toFrame={toFrame}
              toShare={toShare}
              picked={v.isPicked('layer', layer.id)}
              onPick={() => v.toggle('layer', layer.id)}
            />
          </Lane>,
        ]
  );
  const pinLanes = v.pins.map((row, i) => {
    const bars = row.map(layer => (
      <ChangeBar
        key={layer.id}
        layer={layer}
        label={pinLabel(layer)}
        look="bg-sage/70 text-fg"
        frames={frames}
        toFrame={toFrame}
        toShare={toShare}
        picked={v.isPicked('layer', layer.id)}
        onPick={() => v.toggle('layer', layer.id)}
      />
    ));
    return (
      <Lane key={`pin-${i}`} label={i === 0 ? 'Pin' : ''}>
        {bars}
      </Lane>
    );
  });
  return (
    <section
      aria-label="Timeline"
      className="flex flex-col gap-1 rounded-card border border-line bg-panel px-2.5 py-2"
    >
      <div className="flex items-center gap-2">
        <IconButton
          icon={playing ? 'pause' : 'play'}
          label={playing ? 'Pause' : 'Play'}
          size="sm"
          onClick={onToggle}
        />
        <span className="font-mono text-[11.5px] whitespace-nowrap text-fg-dim">
          {frameReadout(frame, frames, fps)}
        </span>
        <span className="flex-1" />
        {v.layers.length || v.pins.length ? (
          <span className="hidden truncate text-meta text-fg-mute 2xl:inline">
            {v.layers.some(l => l.kind === 'pose')
              ? 'Drag a diamond to move a key. Alt drag to copy it.'
              : 'Drag a bar to move it. Drag its ends to change the range.'}
          </span>
        ) : null}
      </div>
      <div className="relative grid" style={{ gridTemplateColumns: `${LABEL}px minmax(0,1fr)` }}>
        <div />
        <div className="relative">
          <input
            type="range"
            aria-label="Playhead"
            min={0}
            max={frames}
            step={1}
            value={frame}
            onChange={e => onScrub(Number(e.target.value))}
            className="block h-4 w-full accent-clay"
          />
          <div aria-hidden className="relative h-5">
            {marks}
          </div>
        </div>
        <Lane label="Base" group />
        <Lane label="Clips">
          {clipBlocks}
          {blends}
        </Lane>
        <Lane label="Trim">{trims}</Lane>
        <Lane label="Retime">{retimes}</Lane>
        <Lane label="Hold">{holds}</Lane>
        <Lane label="Mirror">
          <Block
            block={{
              key: 'mirror',
              first: 0,
              last: frames,
              label: v.mirror
                ? 'On. Left and right swapped'
                : 'Off. Turn on for a left handed copy',
            }}
            frames={frames}
            name={v.mirror ? 'Mirror on' : 'Mirror off'}
            picked={v.isPicked('mirror')}
            onPick={() => v.toggle('mirror')}
            className={
              v.mirror
                ? 'bg-sage/25 text-fg'
                : 'border border-dashed border-line bg-transparent text-fg-mute'
            }
          />
        </Lane>
        {pinLanes.length ? <Lane label="Pins" group /> : null}
        {pinLanes}
        {changes.length ? <Lane label="Changes" group /> : null}
        {changes}
        <i
          aria-hidden
          className="pointer-events-none absolute top-4 bottom-0 w-px bg-fg"
          style={{
            left: `calc(${LABEL}px + (100% - ${LABEL}px) * ${Math.min(Math.max(share, 0), 1)})`,
          }}
        />
      </div>
    </section>
  );
}
