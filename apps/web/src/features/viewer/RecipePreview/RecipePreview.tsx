/**
 * A recipe's base clip on the project's moving model with its changes laid
 * over roughly, live in three.js. The bake solves the same changes exactly
 * in Blender; the label says so on the picture.
 */
import { Suspense } from 'react';
import { StageLoader } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
import { Icon, InlineBanner } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { fileUrl } from '@/ipc/files';
import type { RecipeLayer } from '@/ipc/client';
import type { ReachCheck } from '@/state/recipe-review';
import type { RecipeMarker } from '../recipe-pose';
import { useRoughMap } from '../RoughPreview/RoughPreview.hooks';

const RecipeStage = lazyAfterPaint(() => import('@/features/viewer/RecipeStage'));

export const ROUGH_LABEL = 'Rough preview. The bake solves it exactly.';

export interface RecipePreviewProps {
  project: string | null;
  /** The moving model's asset, its name and its newest file. */
  model: { asset: string; name: string; file: string } | null;
  /** The base clip in the animation library. */
  clip: { id: string; name: string; file: string } | null;
  layers: readonly RecipeLayer[];
  markers: readonly RecipeMarker[];
  share: number;
  playing: boolean;
  onTick: (delta: number) => void;
  onChecks?: (checks: ReachCheck[]) => void;
  /** The words on the picture; the rough preview's own without. */
  label?: string;
  /** The figure's accessible name. */
  name?: string;
  className?: string;
}

const NO_PICKS = [] as const;

function why(props: RecipePreviewProps): string | null {
  if (!props.model) return 'Pick a rigged model in the list to see the recipe on it.';
  if (!props.clip) return 'The base clip is not in the animation library, so nothing can play.';
  return null;
}

export default function RecipePreview(props: RecipePreviewProps) {
  const { model, clip } = props;
  const { map, problem, setFailed } = useRoughMap(
    props.project,
    model?.asset ?? null,
    clip?.id ?? null,
    NO_PICKS
  );
  const modelUrl = fileUrl(model?.file);
  const clipUrl = fileUrl(clip?.file);
  const ready = modelUrl && clipUrl && map && !problem;
  const waiting = why(props);
  return (
    <figure
      aria-label={props.name ?? 'Recipe preview'}
      className={`relative overflow-hidden rounded-card border border-line bg-studio ${props.className ?? ''}`}
    >
      {ready ? (
        <CanvasBoundary resetKey={`${modelUrl}#${clipUrl}`}>
          <Suspense fallback={<StageLoader />}>
            <RecipeStage
              key={`${modelUrl}#${clipUrl}`}
              className="absolute! inset-0"
              label={`${clip?.name ?? 'The base clip'} on ${model?.name ?? 'the model'} with the recipe's changes, rough preview`}
              modelUrl={modelUrl}
              clipUrl={clipUrl}
              pairs={map}
              layers={props.layers}
              markers={props.markers}
              share={props.share}
              playing={props.playing}
              onTick={props.onTick}
              onChecks={props.onChecks}
              onError={setFailed}
            />
          </Suspense>
        </CanvasBoundary>
      ) : null}
      {waiting ? (
        <figcaption className="absolute inset-0 grid place-items-center p-3 text-center text-meta text-fg-mute">
          {waiting}
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
      <span className="pointer-events-none absolute bottom-2 left-2 inline-flex h-6 items-center gap-1.5 rounded-control bg-app/85 px-2 text-[11.5px] leading-none font-medium text-fg">
        <Icon name="info" size={13} className="text-fg-dim" />
        {props.label ?? ROUGH_LABEL}
      </span>
    </figure>
  );
}
