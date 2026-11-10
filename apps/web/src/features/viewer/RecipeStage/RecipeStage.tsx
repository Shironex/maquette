/**
 * A recipe's rough preview: the base clip played live on the project's
 * moving model through the bone map (or the model held in its rest pose), with the recipe's changes laid over it
 * and a small marker on each blank's target. Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import type * as THREE from 'three';
import type { RecipeLayer } from '@/ipc/client';
import type { ReachCheck } from '@/state/recipe-review';
import type { StagePins } from '../recipe-pins';
import type { RecipeMarker } from '../recipe-pose';
import type { RoughPair } from '../rough-retarget';
import { useOrbitControls } from '../ModelPreview/Controls/Controls.hooks';
import PoseGizmo, { type PoseGizmoProps } from '../PoseGizmo';
import { useFraming, useStudioRig } from '../ModelPreview/Studio/Studio.hooks';
import { useMotionModel } from '../MotionStage/Scene/Scene.hooks';
import type { Playable, Timeline } from '@/state/recipe-timeline';
import {
  useJoinedSkeletons,
  useRecipeFrame,
  useRecipeSetup,
  useTargets,
} from './RecipeStage.hooks';
import { SkeletonLoad } from './SkeletonLoad';

/** Marker colours, the theme's clay for a prop and sage for a point, as the editor's blank rows. */
const MARKER_COLOURS = { prop: '#c47d60', point: '#98ae95' } as const;

export interface RecipeStageProps {
  modelUrl: string;
  /** The base clip's file, or null to hold the model's rest pose. */
  clipUrl: string | null;
  pairs: readonly RoughPair[];
  layers: readonly RecipeLayer[];
  markers: readonly RecipeMarker[];
  /**
   * Where the playhead is (a share of the clip, or of the bake's frames for
   * joined clips), whether it plays, and what moves it: seconds since the last frame.
   */
  play: { share: number; playing: boolean; onTick: (delta: number) => void };
  /** A recipe that joins, trims, retimes or mirrors: each clip's file and rate, and the timeline. */
  joined?: { urls: readonly string[]; fps: readonly number[]; timeline: Timeline } | null;
  /**
   * Laid over the changes: pose keys (the clips they stand on, and the gizmo
   * while they are edited), then the pins held last, on the bake's frames.
   */
  laid?: { keys?: StageKeys | null; pins?: StagePins | null };
  onChecks?: (checks: ReachCheck[]) => void;
  onError?: (message: string) => void;
  label: string;
  className?: string;
}

/** The gizmo's props the stage does not fill in itself. */
export type StageGizmo = Omit<PoseGizmoProps, 'rig' | 'size' | 'onDragging'>;

/**
 * A recipe's pose keys on the stage: the clips they stand on (the
 * recipe's), so its pose key changes play, and the turn gizmo with the bone
 * dots while one is edited.
 */
export interface StageKeys {
  on: Playable;
  gizmo: StageGizmo | null;
}

type SceneProps = Omit<RecipeStageProps, 'label' | 'className'>;

function Markers({
  markers,
  targets,
}: {
  markers: readonly RecipeMarker[];
  targets: ReadonlyMap<string, THREE.Vector3>;
}) {
  const balls = markers.map(m => {
    const at = targets.get(m.id);
    if (!at) return null;
    return (
      <mesh key={m.id} position={at} name={`blank ${m.id}`}>
        <sphereGeometry args={[0.035, 16, 12]} />
        <meshBasicMaterial color={MARKER_COLOURS[m.kind]} />
      </mesh>
    );
  });
  return <>{balls}</>;
}

function Scene(props: SceneProps) {
  const { modelUrl, clipUrl, pairs, layers, markers, onError, onChecks, joined } = props;
  const keys = props.laid?.keys;
  const model = useMotionModel(modelUrl, onError);
  const skeleton = useMotionModel(clipUrl, onError);
  const framing = useFraming(model, undefined);
  const rig = useStudioRig(framing);
  const { extra, onLoad, play } = useJoinedSkeletons(clipUrl, skeleton, joined ?? null);
  const setup = useRecipeSetup(model, clipUrl ? skeleton : 'rest', pairs, play);
  const origin = framing && model ? model.scene.position : null;
  const targets = useTargets(markers, framing?.box ?? null, setup?.rig.forward ?? null, origin);
  useRecipeFrame(
    setup,
    layers,
    targets,
    props.play,
    onChecks,
    props.laid?.pins ?? null,
    keys?.on ?? null
  );
  const controls = useOrbitControls(false, framing?.target ?? null);
  const box = framing?.box;
  const size = box ? box.max.y - box.min.y || 1.7 : 1.7;
  const loads = extra.map(url => (
    <SkeletonLoad key={url} url={url} onLoad={onLoad} onError={onError} />
  ));
  return (
    <>
      {loads}
      {rig ? <primitive object={rig} /> : null}
      {model ? <primitive object={model.scene} /> : null}
      <Markers markers={markers} targets={targets} />
      {setup && keys?.gizmo ? (
        <PoseGizmo
          {...keys.gizmo}
          rig={setup.rig}
          size={size}
          onDragging={dragging => {
            controls.enabled = !dragging;
          }}
        />
      ) : null}
    </>
  );
}

export default function RecipeStage(props: RecipeStageProps) {
  return (
    <Canvas
      className={props.className}
      shadows
      camera={{ fov: 30, position: [2, 1.4, 3] }}
      dpr={[1, 2]}
      gl={{ antialias: true }}
      role="img"
      aria-label={props.label}
    >
      <Scene {...props} />
    </Canvas>
  );
}
