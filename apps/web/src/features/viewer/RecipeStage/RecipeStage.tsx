/**
 * A recipe's rough preview: the base clip played live on the project's
 * moving model through the bone map, with the recipe's changes laid over it
 * and a small marker on each blank's target. Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import type * as THREE from 'three';
import type { RecipeLayer } from '@/ipc/client';
import type { ReachCheck } from '@/state/recipe-review';
import type { RecipeMarker } from '../recipe-pose';
import type { RoughPair } from '../rough-retarget';
import { Controls } from '../ModelPreview/Controls';
import { useFraming, useStudioRig } from '../ModelPreview/Studio/Studio.hooks';
import { useMotionModel } from '../MotionStage/Scene/Scene.hooks';
import { useRecipeFrame, useRecipeSetup, useTargets } from './RecipeStage.hooks';

/** Marker colours, the theme's clay for a prop and sage for a point, as the editor's blank rows. */
const MARKER_COLOURS = { prop: '#c47d60', point: '#98ae95' } as const;

export interface RecipeStageProps {
  modelUrl: string;
  clipUrl: string;
  pairs: readonly RoughPair[];
  layers: readonly RecipeLayer[];
  markers: readonly RecipeMarker[];
  /** Where the playhead is, as a share of the clip. */
  share: number;
  playing: boolean;
  /** Seconds since the last frame, while it plays. */
  onTick: (delta: number) => void;
  onChecks?: (checks: ReachCheck[]) => void;
  onError?: (message: string) => void;
  label: string;
  className?: string;
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
  const { modelUrl, clipUrl, pairs, layers, markers, onError, onChecks } = props;
  const model = useMotionModel(modelUrl, onError);
  const skeleton = useMotionModel(clipUrl, onError);
  const framing = useFraming(model, undefined);
  const rig = useStudioRig(framing);
  const setup = useRecipeSetup(model, skeleton, pairs);
  const targets = useTargets(markers, framing?.box ?? null, setup?.rig.forward ?? null);
  useRecipeFrame(setup, layers, targets, props, onChecks);
  return (
    <>
      {rig ? <primitive object={rig} /> : null}
      {model ? <primitive object={model.scene} /> : null}
      <Markers markers={markers} targets={targets} />
      <Controls turntable={false} target={framing?.target ?? null} />
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
