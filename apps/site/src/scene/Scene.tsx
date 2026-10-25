import { CameraRig } from './CameraRig';
import { Callouts } from './Callouts';
import { Environment } from './Environment';
import { Lights } from './Lights';
import { POSTER_CAMERAS, STOPS } from './manifest';
import { Models } from './Models';
import { useProgress } from './Scene.hooks';
import type { SceneProps } from './Workbench.types';

export function Scene(props: SceneProps) {
  const progress = useProgress(props.scroll, props.reducedMotion);
  return (
    <>
      <color attach="background" args={['#1a1917']} />
      <Environment />
      <Lights />
      <Models
        progress={progress}
        reducedMotion={props.reducedMotion}
        firstView={props.initialStop}
        onReady={props.onReady}
        onFailed={props.onFailed}
      />
      <CameraRig
        progress={progress}
        stops={STOPS}
        reducedMotion={props.reducedMotion}
        fixed={
          props.capture && props.capture !== 'stops' ? POSTER_CAMERAS[props.capture] : undefined
        }
      />
      <Callouts progress={progress} reducedMotion={props.reducedMotion} />
    </>
  );
}
