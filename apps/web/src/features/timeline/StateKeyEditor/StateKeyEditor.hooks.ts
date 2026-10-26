import { n } from '@/state/motion';
import { DEFAULT_DURATION, EASINGS } from '@/state/motion-states';
import {
  asKind,
  removeTrigger,
  scrollRange,
  setTrigger,
  stateAt,
  triggerSpan,
} from '@/state/timeline';
import { leadReading, usePageStore } from '@/state/use-page-store';
import { useTimelineStore } from '@/state/use-timeline-store';

/** The selected state change: what drives it, where it is on the page and the move it plays. */
export function useStateKeyEditor() {
  const view = useTimelineStore(s => s.view);
  const draft = useTimelineStore(s => s.draft);
  const selection = useTimelineStore(s => s.selection);
  const edit = useTimelineStore(s => s.edit);
  const scrollMax = leadReading(usePageStore(s => s.readings)).scroll?.max ?? null;
  if (!view || selection?.kind !== 'state') return null;
  const { asset, index } = selection;
  const object = view.states[asset];
  const list = draft.triggers[asset] ?? [];
  const trigger = list[index];
  const model = view.models.find(m => m.asset === asset);
  if (!object || !trigger || !model || (trigger.kind !== 'stop' && trigger.kind !== 'scroll')) {
    return null;
  }
  const span = triggerSpan(trigger, view.stops) ?? { start: 0, end: 0 };
  const others = list.filter((_, i) => i !== index);
  const from = stateAt(object, others, view.stops, span.start - 1e-6);
  const move = object.transitions.find(t => t.from === from && t.to === trigger.to);
  const easing = EASINGS.find(e => e.value === (move?.easing ?? 'ease-out'))?.label ?? '';
  const seconds = move?.duration ?? DEFAULT_DURATION;
  const fromName = object.states.find(s => s.id === from)?.name ?? 'rest';
  const toName = object.states.find(s => s.id === trigger.to)?.name ?? trigger.to;
  const length = n(span.end) - n(span.start);
  const change = (patch: Parameters<typeof setTrigger>[3]) =>
    edit(d => setTrigger(d, asset, index, patch));
  return {
    title: `${model.name} · ${toName}`,
    kind: trigger.kind,
    to: trigger.to,
    stop: trigger.stop ?? '',
    start: n(trigger.start),
    end: n(trigger.end),
    states: object.states.map(s => ({ value: s.id, label: s.name })),
    stops: view.stops.map(s => ({ value: s.id, label: s.name })),
    facts: [
      { label: 'Moves from', value: fromName },
      {
        label: 'On the page',
        value:
          trigger.kind === 'stop'
            ? `Plays once, ${seconds} s, when the camera arrives`
            : 'Scrubs with the scroll',
      },
      { label: 'Easing', value: easing },
      ...(trigger.kind === 'scroll'
        ? [
            {
              label: 'Length on the page',
              value: scrollMax
                ? `${Math.round((length / 100) * scrollMax).toLocaleString('en-US')} px`
                : `${Math.round(length)}% of the page`,
            },
          ]
        : []),
    ],
    setKind: (kind: 'stop' | 'scroll') => {
      if (kind !== trigger.kind) change(asKind(trigger, kind, view.stops));
    },
    setTo: (to: string) => change({ to }),
    setStop: (stop: string) => change({ stop }),
    setStart: (start: number) => change(scrollRange(start, Math.max(n(trigger.end), start + 1))),
    setEnd: (end: number) => change(scrollRange(Math.min(n(trigger.start), end - 1), end)),
    remove: () => edit(d => removeTrigger(d, asset, index), null),
  };
}
