import { useOrbitControls } from '../../ModelPreview/Controls/Controls.hooks';
import { Skeleton } from '../../ModelPreview/Skeleton';
import { useFraming, useStudioRig } from '../../ModelPreview/Studio/Studio.hooks';
import { useRootFollow } from '../../root-follow';
import { useClip, useMotionModel, useOverlays, usePoses } from './Scene.hooks';
import type { SceneProps } from './Scene.types';

/** The model in the studio, posed by its clip or its state, with the overlays asked for. */
export default function Scene(props: SceneProps) {
  const loaded = useMotionModel(props.url, props.report?.onError);
  const framing = useFraming(loaded, props.report?.onLoaded);
  const rig = useStudioRig(framing);
  const extent = framing?.extent ?? 1;
  usePoses(loaded, props.poses);
  const walk = useClip(loaded, props, framing);
  const overlays = useOverlays(loaded, props.outline, extent, props.onFrame);
  const controls = useOrbitControls(props.camera.turntable ?? false, walk.target);
  useRootFollow({
    scene: loaded?.scene ?? null,
    rig,
    framing,
    controls,
    follow: props.camera.follow ?? false,
    frameKey: props.camera.follow !== undefined,
  });
  const skeleton = loaded && framing && props.skeleton && loaded.bones.length > 0;
  return (
    <>
      {rig ? <primitive object={rig} /> : null}
      {loaded ? <primitive object={loaded.scene} /> : null}
      {walk.line ? <primitive object={walk.line} /> : null}
      {overlays ? <primitive object={overlays} /> : null}
      {skeleton ? <Skeleton bones={loaded.bones} extent={extent} /> : null}
    </>
  );
}
