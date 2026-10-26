import { Controls } from '../../ModelPreview/Controls';
import { Skeleton } from '../../ModelPreview/Skeleton';
import { useFraming, useStudioRig } from '../../ModelPreview/Studio/Studio.hooks';
import { useClip, useMotionModel, useOverlays, usePoses } from './Scene.hooks';
import type { SceneProps } from './Scene.types';

/** The model in the studio, posed by its clip or its state, with the overlays asked for. */
export default function Scene(props: SceneProps) {
  const loaded = useMotionModel(props.url, props.onError);
  const framing = useFraming(loaded, undefined);
  const rig = useStudioRig(framing);
  const extent = framing?.extent ?? 1;
  usePoses(loaded, props.poses);
  const walk = useClip(loaded, props, framing);
  const overlays = useOverlays(loaded, props.outline, extent, props.onFrame);
  const skeleton = loaded && framing && props.skeleton && loaded.bones.length > 0;
  return (
    <>
      {rig ? <primitive object={rig} /> : null}
      {loaded ? <primitive object={loaded.scene} /> : null}
      {walk.line ? <primitive object={walk.line} /> : null}
      {overlays ? <primitive object={overlays} /> : null}
      {skeleton ? <Skeleton bones={loaded.bones} extent={extent} /> : null}
      <Controls turntable={props.turntable} target={walk.target} />
    </>
  );
}
