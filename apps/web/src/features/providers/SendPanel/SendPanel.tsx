/**
 * Send to 3D: pick the provider, the images and this send's settings,
 * then send. Opens from a picked concept in the Lab or an asset's card.
 */
import { Button, Icon, InlineBanner } from '@maquette/ui';
import { deepEqual } from '@/lib/deepEqual';
import { SERVICE_LABEL } from '@/state/providers';
import { HandoffView } from '../HandoffView';
import { Sheet } from '@/features/common/Sheet';
import { useSendPanel } from './SendPanel.hooks';
import { SendForm } from './SendForm';

/** The sheet itself, shown while the store has a panel open. */
export default function SendPanel() {
  const { panel, close } = useSendPanel();
  if (!panel) return null;
  const name = panel.options?.asset.name ?? panel.handoff?.assetName ?? 'Asset';
  const handoff = panel.handoff;
  return (
    <Sheet
      id="send"
      icon={handoff ? 'hand-pointing' : panel.redo ? 'arrow-bend-up-right' : 'cube'}
      title={handoff ? `${name}, by hand` : panel.redo ? `Redo ${name}` : `Send ${name} to 3D`}
      meta={
        handoff
          ? `${handoff.id} · ${SERVICE_LABEL[handoff.service]} settings`
          : panel.options
            ? `${panel.options.asset.kind} · ${panel.options.images.length} ${panel.options.images.length === 1 ? 'ref' : 'refs'} in the project`
            : 'Reading the refs'
      }
      onClose={close}
      dirty={Boolean(panel.draft && !deepEqual(panel.draft, panel.initial))}
    >
      {handoff ? (
        <>
          <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
            <HandoffView job={handoff} />
          </div>
          <div className="flex justify-end gap-2 border-t border-line px-5 py-3">
            <Button variant="primary" onClick={close}>
              Done
            </Button>
          </div>
        </>
      ) : panel.error ? (
        <div className="p-5">
          <InlineBanner tone="danger" title="Could not open Send to 3D" detail={panel.error} />
        </div>
      ) : panel.options && panel.draft ? (
        <SendForm options={panel.options} draft={panel.draft} />
      ) : (
        <div className="flex flex-1 items-center justify-center gap-2 text-fg-mute">
          <Icon name="circle-notch" spin size={16} /> Reading the refs
        </div>
      )}
    </Sheet>
  );
}
