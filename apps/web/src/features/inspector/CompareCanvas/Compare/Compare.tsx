import { useCompare } from './Compare.hooks';
import type { CompareProps } from './Compare.types';

/** Everything inside the compare canvas; it draws both sides itself, so it renders nothing. */
export default function Compare(props: CompareProps) {
  useCompare(props);
  return null;
}
