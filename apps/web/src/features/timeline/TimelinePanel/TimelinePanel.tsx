import { Button, InlineBanner, KeyValueList } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { ExplodeKeyEditor } from '../ExplodeKeyEditor';
import { PageKeyEditor } from '../PageKeyEditor';
import { PathKeyEditor } from '../PathKeyEditor';
import { StateKeyEditor } from '../StateKeyEditor';
import { useTimelinePanel } from './TimelinePanel.hooks';

/** The selected change or key, what to look at again with its fixes, and what ships. */
export default function TimelinePanel() {
  const panel = useTimelinePanel();
  if (!panel) return null;
  const checks = panel.checks.map(c => {
    const fixes = (c.fixes ?? []).map(f => (
      <Button key={f.label} size="sm" tooltip={f.tip} onClick={() => panel.fix(f)}>
        {f.label}
      </Button>
    ));
    const detail =
      fixes.length > 0 ? (
        <>
          {c.detail}
          <span className="mt-2 flex flex-wrap gap-1.5">{fixes}</span>
        </>
      ) : (
        c.detail
      );
    return <InlineBanner key={c.key} tone="warning" title={c.title} detail={detail} />;
  });
  return (
    <aside
      aria-label="Selected track"
      className="flex w-[344px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-4"
    >
      {panel.selection === 'state' ? (
        <StateKeyEditor />
      ) : panel.selection === 'explode' ? (
        <ExplodeKeyEditor />
      ) : panel.selection === 'path' ? (
        <PathKeyEditor />
      ) : panel.selection === 'page' ? (
        <PageKeyEditor />
      ) : (
        <p className="text-meta text-fg-mute">
          Pick a change or a key on the timeline to edit it. A plus on a lane adds one at that
          camera stop.
        </p>
      )}
      <section aria-label="Checks" className="flex flex-col gap-2.5 border-t border-line pt-3.5">
        <SectionLabel>Checks · {panel.checks.length}</SectionLabel>
        {checks.length > 0 ? (
          checks
        ) : (
          <p className="text-meta text-fg-mute">Every change does something.</p>
        )}
      </section>
      <section aria-label="On the site" className="flex flex-col gap-3 border-t border-line pt-3.5">
        <SectionLabel>On the site</SectionLabel>
        <KeyValueList items={panel.site} />
        <p className="text-meta text-fg-mute">
          Ship copies the timeline with the layout, and the site plays it as the page scrolls.
          Agents read and write it with get_layout and set_layout.
        </p>
      </section>
    </aside>
  );
}
