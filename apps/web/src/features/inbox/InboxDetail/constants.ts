import type { IconName } from '@maquette/ui';
import type { AssetKind } from '@/ipc/client';
import type { PreviewMode } from '@/features/viewer/ModelPreview';

export const MODES: readonly { value: PreviewMode; label: string }[] = [
  { value: 'shaded', label: 'Shaded' },
  { value: 'parts', label: 'Parts' },
  { value: 'wireframe', label: 'Wireframe' },
  { value: 'uv', label: 'UV' },
  { value: 'bones', label: 'Bones' },
];

export const MODE_ICON: Record<PreviewMode, IconName> = {
  shaded: 'cube',
  parts: 'stack',
  wireframe: 'cube-transparent',
  uv: 'squares-four',
  bones: 'person-simple',
};

/** A label laid over the viewport, as the canvas draws them. */
export const BADGE =
  'pointer-events-none absolute flex h-[22px] items-center gap-[5px] rounded-[6px] bg-[rgba(28,27,25,0.78)] px-2 text-[11.5px] font-medium text-fg';

export const KINDS: readonly { value: AssetKind; label: string }[] = [
  { value: 'prop', label: 'Prop' },
  { value: 'character', label: 'Character' },
  { value: 'scene', label: 'Scene' },
];
