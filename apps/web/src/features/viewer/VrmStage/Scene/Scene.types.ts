import type { VrmStageProps } from '../VrmStage.types';

export type SceneProps = Omit<VrmStageProps, 'className' | 'label'>;
