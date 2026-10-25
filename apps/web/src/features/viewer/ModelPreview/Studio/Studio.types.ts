import type { ModelSource, PreviewMode } from '../ModelPreview.types';

export interface StudioProps {
  source: ModelSource;
  mode: PreviewMode;
  turntable: boolean;
}
