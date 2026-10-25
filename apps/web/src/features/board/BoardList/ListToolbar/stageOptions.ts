import type { SegmentedOption } from '@maquette/ui';
import type { BoardPrefs } from '@/state/board-list';
import { STAGES, STAGE_LABEL } from '@/state/pipeline';
import { STAGE_ICON } from '../../constants';

/** All, then one choice per pipeline stage. */
export const STAGE_OPTIONS: readonly SegmentedOption<BoardPrefs['stage']>[] = [
  { value: 'all', label: 'All' },
  ...STAGES.map(s => ({ value: s, label: STAGE_LABEL[s], icon: STAGE_ICON[s] })),
];
