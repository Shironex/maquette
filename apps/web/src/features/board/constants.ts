import type { IconName } from '@maquette/ui';
import type { Stage } from '@/ipc/client';

/** The drag data type a card carries between columns. */
export const DRAG_TYPE = 'application/x-maquette-asset';

export const STAGE_ICON: Record<Stage, IconName> = {
  concept: 'image',
  refs: 'images',
  tripo: 'cube',
  blender: 'cube-transparent',
  web: 'globe',
  ship: 'rocket-launch',
};
