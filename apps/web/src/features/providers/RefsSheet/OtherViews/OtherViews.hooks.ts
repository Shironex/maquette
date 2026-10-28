import { useState } from 'react';
import type { RefView, RefsView } from '@/ipc/client';
import { DRAWN, candidatesByView, emptyViews } from '@/state/refs';
import { useRefsStore } from '@/state/use-refs-store';

export interface ViewsForm {
  views: RefView[];
  count: number;
  useStyle: boolean;
}

/** The form, the start and the candidates per view of Make the other views. */
export function useOtherViews(view: RefsView) {
  const { makeViews, use, show, busy } = useRefsStore();
  const [form, set] = useState<ViewsForm>(() => ({
    views: emptyViews(view.refs),
    count: 2,
    useStyle: true,
  }));
  const byView = candidatesByView(view.refs, view.directions, view.jobs);
  const groups = DRAWN.map(v => byView[v]);
  const drawing = groups.reduce((n, g) => n + g.drawing, 0);
  const images = form.views.length * form.count;
  const front = view.refs.find(r => r.view === 'front')?.file ?? null;
  const toggleView = (v: RefView, on: boolean) =>
    set(now => ({ ...now, views: on ? [...now.views, v] : now.views.filter(x => x !== v) }));
  return {
    form,
    setForm: (patch: Partial<ViewsForm>) => set(now => ({ ...now, ...patch })),
    start: () => makeViews(form.views, form.count, form.useStyle),
    use,
    back: () => show('refs'),
    busy,
    toggleView,
    front,
    frontName: front ? front.slice(front.lastIndexOf('/') + 1) : '',
    cost: `${images} Codex ${images === 1 ? 'image' : 'images'}, like the Lab`,
    drawLabel: `Draw ${form.views.length} ${form.views.length === 1 ? 'view' : 'views'}`,
    filled: DRAWN.filter(v => view.refs.some(r => r.view === v)) as RefView[],
    byView,
    drawing,
    total: groups.reduce((n, g) => n + g.images.length, 0) + drawing,
  };
}
