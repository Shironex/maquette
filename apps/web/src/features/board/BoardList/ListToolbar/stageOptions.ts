import type { SegmentedOption } from '@maquette/ui';
import type { Stage } from '@/ipc/client';
import type { BoardPrefs } from '@/state/board-list';
import { STAGES } from '@/state/pipeline';
import { STAGE_ICON } from '../../constants';

/** All, then one choice per pipeline stage, named as the project names them. */
export function stageOptions(
  labels: Record<Stage, string>
): readonly SegmentedOption<BoardPrefs['stage']>[] {
  return [
    { value: 'all', label: 'All' },
    ...STAGES.map(s => ({ value: s, label: labels[s], icon: STAGE_ICON[s] })),
  ];
}
