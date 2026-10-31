import { Button, Icon, NumberField, SegmentedControl, Select, Toggle, cn } from '@maquette/ui';
import type { LoopFormat, PackSource, RenderLook } from '@/ipc/client';
import { RenderProblemCard } from '@/features/common/RenderProblemCard';
import { Sheet } from '@/features/common/Sheet';
import { formatBytes } from '@/lib/format';
import {
  FPS,
  LOOP_LABEL,
  LOOK_OPTIONS,
  STYLE_LABEL,
  TARGET_BYTES,
  fileOf,
  secondsAbout,
  shortPath,
} from '@/state/renders';
import { useTurntableSheet } from './TurntableSheet.hooks';

export interface TurntableSheetProps {
  source: PackSource;
  assetId: string;
  name: string;
}

const FORMATS: readonly { value: LoopFormat; label: string }[] = (
  ['webp', 'gif', 'mp4'] as const
).map(value => ({ value, label: LOOP_LABEL[value] }));
const RULES: readonly [string, string][] = [
  ['One palette for the whole loop.', 'Built from all frames, so colours do not flicker.'],
  ['20 fps, each frame 5/100 s.', 'GIF counts in hundredths; other rates round and drift.'],
  ['Keep the moving area small.', 'Pixels outside the box stay still and cost almost nothing.'],
  ['Bayer dithering.', 'An ordered pattern compresses better than error diffusion.'],
];

/** One full turn: the format, the frames, the crop box, the GIF rules and the size. */
export default function TurntableSheet({ source, assetId, name }: TurntableSheetProps) {
  const t = useTurntableSheet(source, assetId);
  if (!t.open) return null;
  const { settings: s, moving } = t;
  const look = STYLE_LABEL[s.look].toLowerCase();
  const percent = Math.min(100, ((t.estimate ?? 0) / TARGET_BYTES) * 100);
  const box = (at: number) => (moving ? `${(at / moving.canvas) * 100}%` : '0');
  const tiles = t.frames.map((url, i) => {
    const number = Math.round((i * t.total) / t.frames.length) + 1;
    return (
      <button
        key={i}
        type="button"
        aria-label={`Sample frame ${i + 1}`}
        onClick={() => t.pick(i)}
        className={cn(
          'relative aspect-square overflow-hidden rounded-control border bg-raised',
          i === t.shown ? 'border-clay-text' : 'border-line hover:border-line-strong'
        )}
      >
        {url ? <img src={url} alt="" className="size-full object-cover" /> : null}
        <span className="absolute bottom-0.5 left-0.5 rounded-[3px] bg-[rgba(16,15,14,0.72)] px-1 font-mono text-[10px] text-[#f5f1ea]">
          {number}
        </span>
      </button>
    );
  });
  const rules = RULES.map(([head, body]) => (
    <li key={head} className="flex gap-2 text-meta text-fg-dim">
      <Icon name="check-circle" size={14} className="mt-0.5 flex-none text-sage" />
      <span>
        <b className="font-semibold text-fg">{head}</b> {body}
      </span>
    </li>
  ));
  const offersCrop = t.over && s.format === 'gif' && !s.tightCrop;
  return (
    <Sheet
      id="turntable"
      icon="arrows-clockwise"
      title={`Turntable · ${name}`}
      meta={`One full turn in ${look}, rendered headless, saved to the project exports`}
      onClose={t.close}
      width={940}
    >
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div className="flex min-w-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          <SegmentedControl
            label="Format"
            options={FORMATS}
            value={s.format}
            onChange={format => t.change({ format })}
          />
          <div className="relative mx-auto aspect-square w-full max-w-[480px] overflow-hidden rounded-[12px] border border-line bg-raised">
            {t.frames[t.shown] ? (
              <img
                src={t.frames[t.shown] ?? undefined}
                alt={`Frame ${t.shown + 1} of the turn`}
                className="size-full object-contain"
              />
            ) : (
              <p className="grid size-full place-items-center text-meta text-fg-mute">
                Asking headless Blender for sample frames...
              </p>
            )}
            {moving && s.format !== 'mp4' ? (
              <div
                aria-label="Moving area"
                className="absolute rounded-[3px] border border-dashed border-focus"
                style={{
                  left: box(moving.x),
                  top: box(moving.y),
                  width: box(moving.w),
                  height: box(moving.h),
                }}
              >
                <span className="absolute -top-px left-1/2 -translate-x-1/2 -translate-y-full rounded-t-[4px] bg-focus px-1.5 text-[11px] whitespace-nowrap text-[#16130f]">
                  Moving area · {moving.w} × {moving.h} px
                </span>
              </div>
            ) : null}
            <span className="absolute bottom-2 left-2 rounded-[5px] bg-[rgba(16,15,14,0.72)] px-2 py-0.5 text-meta text-[#f5f1ea]">
              {t.frameLine}
            </span>
            <button
              type="button"
              onClick={t.togglePlay}
              aria-pressed={t.playing}
              className="absolute right-2 bottom-2 flex items-center gap-1 rounded-[5px] bg-[rgba(16,15,14,0.72)] px-2 py-0.5 text-meta text-[#f5f1ea] hover:bg-[rgba(16,15,14,0.86)]"
            >
              <Icon name={t.playing ? 'pause' : 'play'} size={12} />
              {t.playing ? 'Pause' : 'Plays as saved'}
            </button>
          </div>
          <div className="grid grid-cols-8 gap-1.5">{tiles}</div>
        </div>
        <aside className="flex w-[330px] flex-none flex-col gap-3 overflow-y-auto border-l border-line p-4">
          <div className="grid grid-cols-2 gap-2">
            <NumberField
              label="Length"
              unit="s"
              min={1}
              max={12}
              value={s.length}
              onChange={length => t.change({ length })}
            />
            <div className="flex flex-col gap-1">
              <span className="font-medium">Frame rate</span>
              <span className="py-1.5 font-mono text-fg-dim">{FPS} fps</span>
            </div>
            <NumberField
              label="Width"
              unit="px"
              min={120}
              max={1080}
              value={s.width}
              onChange={width => t.change({ width })}
            />
            <NumberField
              label="Camera tilt"
              unit="°"
              min={0}
              max={60}
              value={s.tilt}
              onChange={tilt => t.change({ tilt })}
            />
          </div>
          <Select<RenderLook>
            label="Style"
            value={s.look}
            options={LOOK_OPTIONS}
            onChange={next => t.change({ look: next })}
          />
          <Toggle
            label="Still background"
            checked={s.stillBackground}
            onChange={stillBackground => t.change({ stillBackground })}
          />
          {s.format === 'gif' ? (
            <div className="flex flex-col gap-2 border-t border-line pt-3">
              <h4 className="text-meta font-medium tracking-wide text-fg-mute uppercase">
                GIF rules
              </h4>
              <ul className="flex flex-col gap-1.5">{rules}</ul>
            </div>
          ) : null}
          {t.needsFfmpeg ? (
            <RenderProblemCard
              name={name}
              style="turntable"
              problem={{
                kind: 'missing-tool',
                message: `${LOOP_LABEL[s.format]} needs ffmpeg. Set its path in Settings, Toolchain, or save a WebP loop, which does not.`,
                webpBytes: null,
              }}
              actions={[
                { label: 'Open Toolchain', icon: 'wrench', onClick: t.openToolchain },
                {
                  label: 'Save as WebP',
                  icon: 'image',
                  onClick: () => t.change({ format: 'webp' }),
                },
              ]}
            />
          ) : null}
          <div className="flex flex-col gap-1 border-t border-line pt-3">
            <div className="flex items-baseline justify-between">
              <span className="font-medium">Estimated size</span>
              <span className={cn('font-mono', t.over ? 'text-amber' : 'text-fg-dim')}>
                {t.estimate ? formatBytes(t.estimate) : '...'} / 2 MB target
              </span>
            </div>
            <div
              role="meter"
              aria-label="Estimated size against the 2 MB target"
              aria-valuemin={0}
              aria-valuemax={TARGET_BYTES}
              aria-valuenow={Math.min(t.estimate ?? 0, TARGET_BYTES)}
              className="h-1.5 overflow-hidden rounded-pill bg-line"
            >
              <div
                className={cn('h-full rounded-pill', t.over ? 'bg-amber' : 'bg-sage')}
                style={{ width: `${percent}%` }}
              />
            </div>
            {offersCrop ? (
              <Button
                size="sm"
                variant="ghost"
                icon="resize"
                onClick={() => t.change({ tightCrop: true })}
              >
                Crop to the model
              </Button>
            ) : null}
            <p className="font-mono text-meta break-all text-fg-mute">
              {t.dir ? shortPath(t.dir, t.folder, fileOf('turntable', s)) : ''}
            </p>
          </div>
        </aside>
      </div>
      <div className="flex items-center gap-2 border-t border-line px-4 py-3">
        <span className="flex flex-1 items-center gap-1.5 text-meta text-fg-mute">
          <Icon name="clock" size={13} />
          About {secondsAbout('turntable', s)} s in headless Blender · no credits
        </span>
        <Button variant="ghost" onClick={t.close}>
          Cancel
        </Button>
        <Button icon="stack" onClick={t.addToBatch}>
          Add to batch
        </Button>
        <Button
          variant="primary"
          icon="arrows-clockwise"
          disabled={t.needsFfmpeg}
          disabledReason="ffmpeg is not found"
          onClick={t.render}
        >
          Render turntable
        </Button>
      </div>
    </Sheet>
  );
}
