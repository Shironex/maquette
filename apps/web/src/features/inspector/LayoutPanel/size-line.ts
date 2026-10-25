import type { LayoutModel } from '@/ipc/client';
import { sizeCm } from '@/state/layout';

/** The model's size at `scale` in centimetres, as `W × H × D cm`. */
export function sizeLine(model: LayoutModel, scale: number): string {
  if (!model.min || !model.max) return 'Size unknown';
  return sizeCm(model, scale)
    .map(v => (v >= 100 ? v.toFixed(0) : v.toFixed(1)))
    .join(' × ')
    .concat(' cm');
}
