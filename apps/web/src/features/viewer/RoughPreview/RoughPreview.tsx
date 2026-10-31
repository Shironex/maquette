/**
 * A library clip played live on a rigged model with the saved bone map.
 * Nothing bakes: three.js does the rough version here, and Blender the exact
 * one in Review.
 */
import { Suspense } from 'react';
import { StageLoader } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
import { InlineBanner } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { fileUrl } from '@/ipc/files';
import type { MapChoice } from '@/ipc/client';
import { useRoughMap } from './RoughPreview.hooks';

const RoughStage = lazyAfterPaint(() => import('@/features/viewer/RoughStage'));

export interface RoughPreviewProps {
  project: string | null;
  asset: string | null;
  modelName: string;
  modelFile: string | null;
  clip: { id: string; name: string; file: string } | null;
  picked?: readonly MapChoice[];
  className?: string;
}

const NO_PICKS: readonly MapChoice[] = [];

export default function RoughPreview(props: RoughPreviewProps) {
  const { clip, modelName } = props;
  const { map, problem, setFailed } = useRoughMap(
    props.project,
    props.asset,
    clip?.id ?? null,
    props.picked ?? NO_PICKS
  );
  const modelUrl = fileUrl(props.modelFile);
  const clipUrl = fileUrl(clip?.file);
  const ready = modelUrl && clipUrl && map && !problem;
  return (
    <figure
      className={`relative overflow-hidden rounded-card border border-line bg-studio ${props.className ?? ''}`}
    >
      {ready ? (
        <CanvasBoundary resetKey={`${modelUrl}#${clipUrl}`}>
          <Suspense fallback={<StageLoader />}>
            <RoughStage
              key={`${modelUrl}#${clipUrl}`}
              className="absolute! inset-0"
              label={`${clip?.name ?? 'A clip'} on ${modelName}, rough preview`}
              modelUrl={modelUrl}
              clipUrl={clipUrl}
              pairs={map}
              onError={setFailed}
            />
          </Suspense>
        </CanvasBoundary>
      ) : null}
      {!clip ? (
        <figcaption className="absolute inset-0 grid place-items-center p-3 text-center text-meta text-fg-mute">
          Tick a clip to see it on {modelName}.
        </figcaption>
      ) : null}
      {problem ? (
        <InlineBanner
          tone="warning"
          title="The preview could not play"
          detail={problem}
          className="absolute inset-x-2 top-2"
        />
      ) : null}
    </figure>
  );
}
