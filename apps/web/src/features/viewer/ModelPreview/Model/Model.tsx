import type { Loaded, PreviewMode } from '../ModelPreview.types';
import { useModelLook } from './Model.hooks';

export interface ModelProps {
  loaded: Loaded;
  mode: PreviewMode;
}

/** The loaded model, drawn in the chosen look. */
export default function Model({ loaded, mode }: ModelProps) {
  useModelLook(loaded, mode);
  return <primitive object={loaded.scene} />;
}
