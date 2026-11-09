/**
 * A recipe's base clip on the project's moving model with its changes laid
 * over roughly, live in three.js. The bake solves the same changes exactly
 * in Blender; the label says so on the picture.
 */
import { Suspense } from 'react';
import type { Timeline } from '@/state/recipe-timeline';
import type { StageKeys } from '@/features/viewer/RecipeStage';
import { StageLoader } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
import { Icon, InlineBanner } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { fileUrl } from '@/ipc/files';
import type { RecipeLayer } from '@/ipc/client';
import type { ReachCheck } from '@/state/recipe-review';
import type { StagePins } from '../recipe-pins';
import type { RecipeMarker } from '../recipe-pose';
import { useRoughMap } from '../RoughPreview/RoughPreview.hooks';
import { useRecipePreview } from './RecipePreview.hooks';

const RecipeStage = lazyAfterPaint(() => import('@/features/viewer/RecipeStage'));

export const ROUGH_LABEL = 'Rough preview. The bake solves it exactly.';

/** A clip of a joined recipe as the library has it, and its frames a second. */
export interface JoinedClip {
  id: string | null;
  file: string | null;
  fps: number;
}

/** A recipe that joins, trims, retimes or mirrors: every clip, base first, and its timeline. */
export interface JoinedClips {
  joined: readonly JoinedClip[];
  timeline: Timeline;
}

export interface RecipePreviewProps {
  project: string | null;
  /** The moving model's asset, its name and its newest file. */
  model: { asset: string; name: string; file: string } | null;
  /**
   * The base clip in the animation library, `'rest'` for the model's rest
   * pose, or every clip of a recipe that joins, trims, retimes or mirrors.
   */
  clip: { id: string; name: string; file: string } | 'rest' | JoinedClips | null;
  layers: readonly RecipeLayer[];
  markers: readonly RecipeMarker[];
  /**
   * Where the playhead is (a share of the clip, or of the bake's frames for
   * joined clips), whether it plays, and who moves it on.
   */
  play: { share: number; playing: boolean; onTick: (delta: number) => void };
  onChecks?: (checks: ReachCheck[]) => void;
  /** The pins to hold last, on the bake's frames. */
  pins?: StagePins | null;
  /** Pose keys: the clips they stand on, and the turn gizmo while they are edited. */
  keys?: StageKeys | null;
  /** The words on the picture; the rough preview's own without. */
  label?: string;
  /** The figure's accessible name. */
  name?: string;
  className?: string;
}

const NO_PICKS = [] as const;

function why(props: RecipePreviewProps): string | null {
  if (!props.model) return 'Pick a rigged model in the list to see the recipe on it.';
  const { clip } = props;
  const lost =
    clip && typeof clip === 'object' && 'joined' in clip ? clip.joined.findIndex(c => !c.file) : -1;
  if (lost >= 0) return `Clip ${lost + 1} is not in the animation library, so nothing can play.`;
  if (!clip) return 'The base clip is not in the animation library, so nothing can play.';
  return null;
}

export default function RecipePreview(props: RecipePreviewProps) {
  const { model } = props;
  const v = useRecipePreview(props.clip);
  const { map, problem, setFailed } = useRoughMap(
    props.project,
    model?.asset ?? null,
    v.clipId,
    NO_PICKS,
    v.rest
  );
  const modelUrl = fileUrl(model?.file);
  const ready = modelUrl && (v.rest || v.clipUrl) && v.playable && map && !problem;
  // A new set of clip files is a new stage; trims and time edits only pose it again.
  const stageKey = `${modelUrl}#${v.files}`;
  const waiting = why(props);
  return (
    <figure
      aria-label={props.name ?? 'Recipe preview'}
      className={`relative overflow-hidden rounded-card border border-line bg-studio ${props.className ?? ''}`}
    >
      {ready ? (
        <CanvasBoundary resetKey={stageKey}>
          <Suspense fallback={<StageLoader />}>
            <RecipeStage
              key={stageKey}
              className="absolute! inset-0"
              label={`${v.base} on ${model?.name ?? 'the model'} with the recipe's changes, rough preview`}
              modelUrl={modelUrl}
              clipUrl={v.clipUrl}
              joined={v.play}
              pairs={map}
              layers={props.layers}
              markers={props.markers}
              play={props.play}
              onChecks={props.onChecks}
              pins={props.pins}
              keys={props.keys}
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
