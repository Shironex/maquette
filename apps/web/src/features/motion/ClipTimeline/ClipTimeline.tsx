import type { PointerEvent } from 'react';
import { Icon, IconButton, Kbd, Select, Tooltip } from '@maquette/ui';
import { SPEEDS, type Speed } from '@/state/motion';
import { frameUnder, useClipTimeline } from './ClipTimeline.hooks';

const at = (frame: number, frames: number) => `${frames > 0 ? (frame / frames) * 100 : 0}%`;

const HINTS: readonly [readonly string[], string][] = [
  [['Space'], 'play'],
  [[',', '.'], 'step a frame'],
  [['I', 'O'], 'set in and out'],
  [['L'], 'loop'],
];

/** Play, scrub and loop the clip; In and Out set the preview loop, the clip ships whole. */
export default function ClipTimeline() {
  const { clip, player, frame, frames, readout, ticks, fps, controls, trimmedFrom } =
    useClipTimeline();
  if (!clip) return null;
  const [from, to] = player.range;
  const step = fps > 0 && frames / fps > 4 ? 1 : 0.5;
  const seconds: number[] = [];
  for (let sec = step; fps > 0 && sec * fps <= frames + 1e-6; sec += step) seconds.push(sec);
  const grab = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture?.(e.pointerId);
    controls.seek(frameUnder(e, frames));
  };
  const scrub = (e: PointerEvent<HTMLDivElement>) => {
    if (e.buttons === 1) controls.seek(frameUnder(e, frames));
  };
  const drag = (end: 'in' | 'out') => (e: PointerEvent<HTMLButtonElement>) => {
    const lane = e.currentTarget.parentElement;
    if (!lane || e.buttons !== 1) return;
    const box = lane.getBoundingClientRect();
    const f = Math.round(
      Math.min(Math.max((e.clientX - box.left) / (box.width || 1), 0), 1) * frames
    );
    controls.setRange(end === 'in' ? [Math.min(f, to - 1), to] : [from, Math.max(f, from + 1)]);
  };
  const span = to - from;
  const share = frames > 0 ? frame / frames : 0;
  const tickMarks = ticks.map(t => (
    <span key={t}>
      <i className="absolute top-0 h-2 w-px bg-fg-mute" style={{ left: at(t, frames) }} />
      <span
        className="absolute top-[9px] -translate-x-1/2 whitespace-nowrap"
        style={{ left: at(t, frames) }}
      >
        {t}
      </span>
    </span>
  ));
  const secondMarks = seconds.map(sec => (
    <span
      key={sec}
      className="absolute top-[21px] -translate-x-1/2 whitespace-nowrap"
      style={{ left: at(sec * fps, frames) }}
    >
      {sec}s
    </span>
  ));
  const handles = (['in', 'out'] as const).map(end => (
    <Tooltip
      key={end}
      label={
        end === 'in' ? 'Drag to set where the preview loop starts' : 'Drag to set where it ends'
      }
    >
      <button
        type="button"
        aria-label={end === 'in' ? `In, frame ${from}` : `Out, frame ${to}`}
        onPointerDown={e => e.currentTarget.setPointerCapture?.(e.pointerId)}
        onPointerMove={drag(end)}
        className="absolute top-[3px] h-7 w-2 -translate-x-1/2 cursor-ew-resize rounded-[3px] bg-fg"
        style={{ left: at(end === 'in' ? from : to, frames) }}
      />
    </Tooltip>
  ));
  const hints = HINTS.map(([keys, what]) => {
    const kbds = keys.map(k => <Kbd key={k} keys={k} />);
    return (
      <span key={what} className="inline-flex items-center gap-1.5">
        {kbds}
        {what}
      </span>
    );
  });
  const speeds = SPEEDS.map(s => ({ value: s, label: `${s}x` }));
  return (
    <section
      aria-label="Timeline"
      className="flex flex-none flex-col rounded-card border border-line bg-panel"
    >
      <div className="flex items-center gap-2 border-b border-line px-2.5 py-2">
        <IconButton icon="skip-back" label="Go to In" size="sm" onClick={controls.toStart} />
        <IconButton
          icon={player.playing ? 'pause' : 'play'}
          label={player.playing ? 'Pause' : 'Play'}
          size="sm"
          shortcut="Space"
          onClick={controls.toggle}
        />
        <IconButton icon="skip-forward" label="Go to Out" size="sm" onClick={controls.toEnd} />
        <IconButton
          icon="repeat"
          label={player.loop ? 'Loop on' : 'Loop off'}
          size="sm"
          shortcut="L"
          variant={player.loop ? 'primary' : 'secondary'}
          aria-pressed={player.loop}
          onClick={() => controls.setLoop(!player.loop)}
        />
        <Select
          aria-label="Speed"
          size="sm"
          inline
          options={speeds}
          value={player.speed}
          onChange={(s: Speed) => controls.setSpeed(s)}
          fieldClassName="w-[72px]"
        />
        <span className="ml-2 font-mono text-[12.5px] text-fg" aria-live="off">
          {readout}
        </span>
        <span className="ml-auto text-meta text-fg-dim">
          {trimmedFrom ? `Trimmed from ${trimmedFrom} f in the library · ` : ''}Loop {from} to {to}
        </span>
      </div>
      <div className="relative grid grid-cols-[132px_1fr]">
        <div />
        <div
          className="relative h-[34px] cursor-pointer border-l border-line font-mono text-[10.5px] leading-none text-fg-mute"
          onPointerDown={grab}
          onPointerMove={scrub}
          role="slider"
          tabIndex={0}
          aria-label="Playhead"
          aria-valuemin={0}
          aria-valuemax={frames}
          aria-valuenow={frame}
          aria-valuetext={readout}
        >
          {tickMarks}
          {secondMarks}
        </div>
        <div className="flex h-[34px] items-center gap-1.5 overflow-hidden border-t border-line px-3 text-[12px] whitespace-nowrap text-fg-dim">
          <Icon name="film-strip" size={14} className="text-fg-mute" />
          {clip.name}
        </div>
        <div className="relative h-[34px] border-t border-l border-line">
          <div
            className="absolute top-1.5 flex h-[22px] items-center gap-1.5 overflow-hidden rounded-[5px] border border-clay-line bg-clay/20 px-2 text-[11.5px] whitespace-nowrap text-fg"
            style={{ left: at(from, frames), width: at(span, frames) }}
          >
            <Icon name="repeat" size={12} />
            {clip.name} · {span} f
          </div>
          {handles}
        </div>
        <div
          aria-hidden="true"
          className="pointer-events-none absolute top-0 bottom-0 w-0.5 bg-clay-text"
          style={{ left: `calc(132px + (100% - 132px) * ${share})` }}
        />
      </div>
      <div className="flex flex-wrap items-center gap-3 border-t border-line px-3 py-2 text-[11.5px] text-fg-mute">
        {hints}
        <span className="ml-auto">Drag the white handles to set the preview loop</span>
      </div>
    </section>
  );
}
