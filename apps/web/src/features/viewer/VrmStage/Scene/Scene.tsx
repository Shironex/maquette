import { Controls } from '../../ModelPreview/Controls';
import { useStudioRig } from '../../ModelPreview/Studio/Studio.hooks';
import { useIdle, useVrm } from './Scene.hooks';
import type { SceneProps } from './Scene.types';

/** The VRM in the studio, idling, its eyes on the pointer when asked. */
export default function Scene(props: SceneProps) {
  const loaded = useVrm(props.url, props.onLoaded, props.onError);
  const rig = useStudioRig(loaded?.framing ?? null);
  useIdle(loaded?.vrm ?? null, props);
  return (
    <>
      {rig ? <primitive object={rig} /> : null}
      {loaded ? <primitive object={loaded.vrm.scene} /> : null}
      <Controls turntable={false} target={loaded?.framing.target ?? null} />
    </>
  );
}
