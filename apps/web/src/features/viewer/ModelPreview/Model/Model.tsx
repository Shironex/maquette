import type { ThreeEvent } from '@react-three/fiber';
import { partOf, type PartLook } from '../../parts';
import type { Loaded, PreviewMode } from '../ModelPreview.types';
import { useModelLook } from './Model.hooks';

export interface ModelProps {
  loaded: Loaded;
  mode: PreviewMode;
  parts?: PartLook;
  onPickPart?: (part: string | null, adding: boolean) => void;
}

/** The loaded model, drawn in the chosen look; a click picks a part when asked. */
export default function Model({ loaded, mode, parts, onPickPart }: ModelProps) {
  useModelLook(loaded, mode, parts);
  const pick = onPickPart
    ? (e: ThreeEvent<MouseEvent>) => {
        e.stopPropagation();
        onPickPart(partOf(e.object), e.shiftKey);
      }
    : undefined;
  return <primitive object={loaded.scene} onClick={pick} />;
}
