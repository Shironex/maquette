import type { ModelPreviewProps, ModelSource, PreviewMode } from '../ModelPreview.types';

export interface StudioProps {
  source: ModelSource;
  mode: PreviewMode;
  turntable: boolean;
  parts?: ModelPreviewProps['parts'];
  onPickPart?: ModelPreviewProps['onPickPart'];
}
