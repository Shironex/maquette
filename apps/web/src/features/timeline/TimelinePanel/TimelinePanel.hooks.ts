import { siteSummary, timelineChecks } from '@/state/timeline-checks';
import { useTimelineStore } from '@/state/use-timeline-store';

/** The side panel: the selection's editor, the checks and what the site gets. */
export function useTimelinePanel() {
  const view = useTimelineStore(s => s.view);
  const draft = useTimelineStore(s => s.draft);
  const selection = useTimelineStore(s => s.selection);
  if (!view) return null;
  const { lanes, marks } = siteSummary(view, draft);
  return {
    selection: selection?.kind ?? null,
    checks: timelineChecks(view, draft),
    site: [
      { label: 'Ships in', value: 'layout.json', mono: true },
      { label: 'Tracks', value: `${lanes} ${lanes === 1 ? 'lane' : 'lanes'}, ${marks} marks` },
      { label: 'Plays', value: 'With the scroll' },
      { label: 'Reduced motion', value: 'Shows the posters' },
    ],
  };
}
