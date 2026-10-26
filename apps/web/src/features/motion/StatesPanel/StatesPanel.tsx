import { KeyValueList } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { PoseEditor } from '../PoseEditor';
import { TransitionEditor } from '../TransitionEditor';
import { TriggerList } from '../TriggerList';
import { useStatesPanel } from './StatesPanel.hooks';

/** The selected state: the move into it, its pose, the triggers and where it is kept. */
export default function StatesPanel() {
  const panel = useStatesPanel();
  return (
    <aside
      aria-label="State details"
      className="flex w-[344px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-4"
    >
      {panel ? (
        <>
          <TransitionEditor />
          <PoseEditor />
          <div className="border-t border-line pt-3.5">
            <TriggerList />
          </div>
          <section
            aria-label="On the site"
            className="flex flex-col gap-3 border-t border-line pt-3.5"
          >
            <SectionLabel>On the site</SectionLabel>
            <KeyValueList items={panel.items} />
            <p className="text-meta text-fg-mute">
              States are saved in layout.json next to the camera stops, and agents read and write
              them with get_layout and set_layout. The site plays the stop and scroll triggers as
              the page scrolls; click and hover do not play there yet.
            </p>
          </section>
        </>
      ) : (
        <p className="text-meta text-fg-mute">
          States pose the parts of a model, and triggers on the site move between them. Give the
          model states to start.
        </p>
      )}
    </aside>
  );
}
