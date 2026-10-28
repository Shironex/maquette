import type { AssembleStageProps } from '../AssembleStage.types';
import { usePlacer } from './Placer.hooks';

/** Everything inside the assembly canvas; it draws through three directly, so it renders nothing. */
export default function Placer(props: Omit<AssembleStageProps, 'className'>) {
  usePlacer(props);
  return null;
}
