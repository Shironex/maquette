import type { LayoutView } from '@/ipc/client';
import type { PlainLayout } from '@/state/layout';
import type { Selection } from '@/state/use-layout-store';
import { ModelDetails } from '../ModelDetails';
import { StopDetails } from '../StopDetails';

export interface SelectedProps {
  view: LayoutView;
  layout: PlainLayout;
  selection: Selection;
}

/** The fields of the selected model or stop, or how to select one. */
export default function Selected({ view, layout, selection }: SelectedProps) {
  if (selection?.kind === 'model') {
    const model = view.models.find(m => m.asset === selection.id);
    if (model) return <ModelDetails model={model} layout={layout} />;
  }
  if (selection?.kind === 'stop') {
    const index = layout.stops.findIndex(s => s.id === selection.id);
    const stop = layout.stops[index];
    if (stop) {
      return (
        <StopDetails stop={stop} index={index} count={layout.stops.length} models={view.models} />
      );
    }
  }
  return (
    <p className="text-meta text-fg-mute">
      Click a model or a camera in the view, or pick one below. W, E and R switch between move, turn
      and scale.
    </p>
  );
}
