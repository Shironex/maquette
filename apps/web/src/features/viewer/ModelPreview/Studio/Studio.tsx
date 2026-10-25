import { Controls } from '../Controls';
import { Model } from '../Model';
import { Skeleton } from '../Skeleton';
import { useFraming, useModel, useStudioRig } from './Studio.hooks';
import type { StudioProps } from './Studio.types';

/** The model in the studio: the rig, the model in its look, its skeleton and the orbit. */
export default function Studio({ source, mode, turntable }: StudioProps) {
  const loaded = useModel(source.url, source.onError);
  const framing = useFraming(loaded, source.onLoaded);
  const rig = useStudioRig(framing);
  const skeleton =
    loaded && framing && mode === 'bones' && loaded.bones.length > 0
      ? { bones: loaded.bones, extent: framing.extent }
      : null;
  return (
    <>
      {rig ? <primitive object={rig} /> : null}
      {loaded ? <Model loaded={loaded} mode={mode} /> : null}
      {skeleton ? <Skeleton bones={skeleton.bones} extent={skeleton.extent} /> : null}
      <Controls turntable={turntable} target={framing?.target ?? null} />
    </>
  );
}
