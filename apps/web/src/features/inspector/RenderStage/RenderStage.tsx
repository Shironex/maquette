import { InlineBanner, SegmentedControl, cn } from '@maquette/ui';
import type { RenderCamera } from '@/ipc/client';
import { STYLE_LABEL } from '@/state/renders';
import { useRenderStage } from './RenderStage.hooks';

const CAMERAS: readonly { value: RenderCamera; label: string }[] = [
  { value: 'front', label: 'Front' },
  { value: 'three-quarter', label: 'Three quarter' },
  { value: 'side', label: 'Side' },
];

const TAG = 'rounded-[5px] bg-[rgba(16,15,14,0.72)] px-2 py-0.5 text-meta text-[#f5f1ea]';

/** The render on show, its camera and the strip of the other pictures. */
export default function RenderStage() {
  const s = useRenderStage();
  const label = STYLE_LABEL[s.style];
  const frames = s.frames.map((url, i) => (
    <img key={i} src={url ?? undefined} alt="" className="size-full object-contain" />
  ));
  const tiles = s.tiles.map(t => (
    <button
      key={t.key}
      type="button"
      title={t.label}
      onClick={t.onClick}
      className={cn(
        'relative aspect-square overflow-hidden rounded-control border bg-raised',
        t.active ? 'border-clay-text' : 'border-line hover:border-line-strong'
      )}
    >
      {t.url ? <img src={t.url} alt="" className="size-full object-cover" /> : null}
      <span className="absolute bottom-0.5 left-0.5 rounded-[3px] bg-[rgba(16,15,14,0.72)] px-1 font-mono text-[10px] text-[#f5f1ea]">
        {t.label}
      </span>
    </button>
  ));
  return (
    <section aria-label="Render style" className="flex min-w-0 flex-1 flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <h2 className="text-panel-title">Render style</h2>
        <span className="rounded-pill bg-clay-soft px-2 text-meta text-clay-text">
          {label.toLowerCase()}
        </span>
        <span className="flex-1" />
        <SegmentedControl
          label="Camera"
          options={CAMERAS}
          value={s.camera}
          onChange={s.setCamera}
        />
      </div>
      <div
        aria-busy={s.loading}
        style={s.stageUrl ? { backgroundColor: s.paper } : undefined}
        className="relative grid min-h-0 flex-1 place-items-center overflow-hidden rounded-[12px] border border-line bg-raised"
      >
        {s.style === 'contact-sheet' && s.frames.length > 0 ? (
          <div
            role="img"
            aria-label={`Contact sheet of ${s.preview?.name ?? 'the model'}`}
            className="grid size-full grid-cols-4 grid-rows-2"
          >
            {frames}
          </div>
        ) : s.stageUrl ? (
          <img
            src={s.stageUrl}
            alt={`${label} of ${s.preview?.name ?? 'the model'}`}
            className={cn(
              'size-full object-contain transition-opacity duration-120',
              s.loading && 'opacity-60'
            )}
          />
        ) : !s.error ? (
          <p className="text-meta text-fg-mute">Asking headless Blender for a picture...</p>
        ) : null}
        {s.error ? (
          <div className="absolute inset-x-4 top-4">
            <InlineBanner tone="danger" title="No picture" detail={s.error} />
          </div>
        ) : null}
        {s.taken ? <span className={cn(TAG, 'absolute top-2.5 left-2.5')}>{s.taken}</span> : null}
        {s.title ? (
          <span className={cn(TAG, 'absolute bottom-2.5 left-2.5 font-mono')}>{s.title}</span>
        ) : null}
      </div>
      <div className="grid flex-none grid-cols-7 gap-1.5">{tiles}</div>
    </section>
  );
}
