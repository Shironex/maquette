import { CameraRig } from './CameraRig';
import { Callouts } from './Callouts';
import { Environment } from './Environment';
import { Lights } from './Lights';
import { POSTER_CAMERAS, STOPS } from './manifest';
import { Models } from './Models';
import { Motes } from './Motes';
import { PathTubes } from './PathTubes';
import { useAdaptiveDpr, useLiveFrames, useProgress } from './Scene.hooks';
import type { SceneProps } from './Workbench.types';

export function Scene(props: SceneProps) {
  // Poster capture holds exact frames: no easing, breathing or idle motion.
  const still = Boolean(props.capture);
  const progress = useProgress(props.scroll, still);
  useLiveFrames(still);
  useAdaptiveDpr(props.scroll, still);
  return (
    <>
      <color attach="background" args={['#1a1917']} />
      <Environment />
      <Lights still={still} />
      <Models
        progress={progress}
        still={still}
        firstView={props.initialStop}
        onReady={props.onReady}
        onFailed={props.onFailed}
      />
      <PathTubes progress={progress} />
      <Motes still={still} />
      <CameraRig
        progress={progress}
        stops={STOPS}
        still={still}
        fixed={
          props.capture && props.capture !== 'stops' ? POSTER_CAMERAS[props.capture] : undefined
        }
      />
      <Callouts progress={progress} />
    </>
  );
}
