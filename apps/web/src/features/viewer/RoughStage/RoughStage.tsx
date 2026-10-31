/**
 * A library clip played live on a rigged model, roughly, before anything
 * bakes: the Choose step's preview and Try on in the Animations library.
 * Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import type { RoughPair } from '../rough-retarget';
import { Controls } from '../ModelPreview/Controls';
import { useFraming, useStudioRig } from '../ModelPreview/Studio/Studio.hooks';
import { useMotionModel } from '../MotionStage/Scene/Scene.hooks';
import { useRoughPose } from './RoughStage.hooks';

export interface RoughStageProps {
  modelUrl: string;
  clipUrl: string;
  pairs: readonly RoughPair[];
  onFrame?: (seconds: number) => void;
  onError?: (message: string) => void;
  label: string;
  className?: string;
}

type SceneProps = Omit<RoughStageProps, 'label' | 'className'>;

function Scene({ modelUrl, clipUrl, pairs, onFrame, onError }: SceneProps) {
  const model = useMotionModel(modelUrl, onError);
  const skeleton = useMotionModel(clipUrl, onError);
  const framing = useFraming(model, undefined);
  const rig = useStudioRig(framing);
  useRoughPose(model, skeleton, pairs, onFrame);
  return (
    <>
      {rig ? <primitive object={rig} /> : null}
      {model ? <primitive object={model.scene} /> : null}
      <Controls turntable={false} target={framing?.target ?? null} />
    </>
  );
}

export default function RoughStage(props: RoughStageProps) {
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
