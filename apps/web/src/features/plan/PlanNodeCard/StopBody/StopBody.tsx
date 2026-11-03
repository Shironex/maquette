import { cn, Icon } from '@maquette/ui';
import type { PlanNode } from '@/ipc/client';
import { useStopBody } from './StopBody.hooks';

/**
 * The body of a camera stop node: its place in the tour, its field of
 * view, the poster render (or a frame saying there is none yet), its name
 * and one quiet note (first view, last, moved from stop 4, still stop 2, new angle).
 */
export default function StopBody({ node }: { node: PlanNode }) {
  const v = useStopBody(node);
  if (!v.stop) return null;
  return (
    <>
      <div className="flex items-center gap-1.5 px-2.5 pt-2 text-[10.5px] leading-[14px] font-medium tracking-[0.05em] text-fg-mute uppercase">
        <Icon name="video-camera" size={12} />
        <span className={cn(v.main && 'text-clay-text')}>{v.label}</span>
        <span className="ml-auto font-mono text-[11px] tracking-normal normal-case">
          {Math.round(v.stop.fov ?? 0)}°
        </span>
      </div>
      <div className="mx-2 mt-2 aspect-video overflow-hidden rounded-[7px] bg-studio">
        {v.src ? (
          <img
            src={v.src}
            alt={`Render of ${v.stop.name}`}
            className="block size-full object-cover"
            draggable={false}
          />
        ) : (
          <div className="grid size-full place-items-center border border-dashed border-line-strong text-meta text-fg-mute">
            No render yet
          </div>
        )}
      </div>
      <p className="truncate px-2.5 pt-1.5 text-[13px] leading-[17px] font-semibold text-fg">
        {v.stop.name}
      </p>
      <p className="truncate px-2.5 pb-2.5 text-[11.5px] leading-4 text-fg-mute">{v.note || '​'}</p>
    </>
  );
}
