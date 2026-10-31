/**
 * The sends of the open project in one sheet: the 3D queue and the By hand
 * checklist, as tabs while both have something to say. Opens from the
 * Board's Sends button and menu, and after a By hand batch.
 */
import { Button, Tabs } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import type { SendsTab } from '@/state/sends';
import { HandoffChecklist } from '../HandoffChecklist';
import { ProviderQueue } from '../ProviderQueue';
import { useSendsPanel } from './SendsPanel.hooks';

export default function SendsPanel() {
  const { open, tab, showTabs, inFlight, waiting, meta, setTab, close } = useSendsPanel();
  if (!open) return null;
  return (
    <Sheet
      id="sends"
      icon={tab === 'hand' ? 'hand-pointing' : 'cube'}
      title={tab === 'hand' ? 'By hand checklist' : '3D queue'}
      meta={meta}
      onClose={close}
      width={600}
    >
      {showTabs ? (
        <Tabs
          tabs={[
            { id: 'queue', label: '3D queue', count: inFlight },
            { id: 'hand', label: 'By hand', count: waiting },
          ]}
          value={tab}
          onChange={id => setTab(id as SendsTab)}
          className="flex-none px-5"
        />
      ) : null}
      {tab === 'hand' ? <HandoffChecklist /> : <ProviderQueue onClose={close} />}
      <div className="flex justify-end gap-2 border-t border-line px-5 py-3">
        <Button variant="primary" onClick={close}>
          Done
        </Button>
      </div>
    </Sheet>
  );
}
