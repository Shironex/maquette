import type { LayoutModel } from '@/ipc/client';
import { dropToFloor, placementOf, type PlainLayout, type PlainPlacement } from '@/state/layout';
import { useLayoutStore } from '@/state/use-layout-store';

/** The model's placement in the draft, and edits to it. */
export function useModelDetails(model: LayoutModel, layout: PlainLayout) {
  const placeModel = useLayoutStore(s => s.placeModel);
  const p = placementOf(layout, model);
  const place = (next: Partial<PlainPlacement>) => placeModel(model.asset, { ...p, ...next });
  const dropToDesk = () => placeModel(model.asset, dropToFloor(model, p));
  return { p, place, dropToDesk };
}
