/**
 * A library clip playing on its own skeleton, looping, in the Motion
 * view's stage. The clip file holds bones and keys only, so the stage
 * draws the skeleton and the root's trail.
 */
import { Suspense, lazy } from 'react';
import { Icon, InlineBanner } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import type { AnimationItem } from '@/ipc/client';
import { useClipPreview } from './ClipPreview.hooks';

const MotionStage = lazy(() => import('@/features/viewer/MotionStage'));

const CHIP =
  'inline-flex h-6 items-center gap-1.5 rounded-control bg-app/85 px-2 text-[11.5px] leading-none font-medium text-fg';

export interface ClipPreviewProps {
  item: AnimationItem;
  className?: string;
}

export default function ClipPreview({ item, className }: ClipPreviewProps) {
  const { clip } = item;
  const { url, time, frame, travel, failed, setFailed, tick } = useClipPreview(item);
  return (
    <div
      className={`relative overflow-hidden rounded-card border border-line bg-studio ${className ?? ''}`}
    >
      {url && !failed ? (
        <CanvasBoundary resetKey={url}>
          <Suspense fallback={null}>
            <MotionStage
              className="absolute! inset-0"
              label={`${clip.name} on its own skeleton, frame ${frame}`}
              url={url}
              clip={0}
              time={time}
              skeleton
              trail={Boolean(clip.rootMotion)}
              turntable={false}
              poses={null}
              outline={[]}
              onFrame={tick}
              onError={setFailed}
            />
          </Suspense>
        </CanvasBoundary>
      ) : null}
      {failed ? (
        <InlineBanner
          tone="danger"
          title="The clip could not be shown"
          detail={failed}
          className="absolute inset-x-2 top-2"
        />
      ) : null}
      <div className="pointer-events-none absolute top-2 left-2 flex gap-1.5">
        <span className={CHIP}>
          <Icon name="film-strip" size={13} className="text-fg-dim" />
          frame {frame} / {clip.frames}
        </span>
        {travel ? (
          <span className={CHIP}>
            <Icon name="path" size={13} className="text-fg-dim" />
            {travel}
          </span>
        ) : null}
      </div>
      <span
        className={`${CHIP} pointer-events-none absolute right-2 bottom-2 font-mono text-fg-dim`}
      >
        {time.toFixed(2)} s
      </span>
    </div>
  );
}
