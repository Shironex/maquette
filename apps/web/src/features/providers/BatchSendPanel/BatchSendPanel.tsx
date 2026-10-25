/**
 * Send several assets to 3D in one go: one provider and one set of
 * presets for all, a kind per asset, one model per asset. An API batch
 * asks once before it spends credits; a By hand batch writes one handoff
 * folder per asset and opens the checklist.
 */
import { Icon, InlineBanner } from '@maquette/ui';
import { deepEqual } from '@/lib/deepEqual';
import { batchSummary } from '@/state/batch';
import { Sheet } from '../Sheet';
import { BatchForm } from './BatchForm';
import { useBatchPanel } from './BatchSendPanel.hooks';

export default function BatchSendPanel() {
  const { batch, close } = useBatchPanel();
  if (!batch) return null;
  const n = batch.assets.length;
  return (
    <Sheet
      id="batch"
      icon={batch.redo ? 'arrow-bend-up-right' : 'stack'}
      title={
        batch.redo
          ? `Redo ${n} ${n === 1 ? 'model' : 'models'}`
          : `Send ${n} ${n === 1 ? 'asset' : 'assets'} to 3D`
      }
      meta={batch.draft ? batchSummary(batch.draft) : 'Reading the refs'}
      onClose={close}
      dirty={Boolean(batch.draft && !deepEqual(batch.draft, batch.initial))}
      width={560}
    >
      {batch.error ? (
        <div className="p-5">
          <InlineBanner tone="danger" title="Could not open the batch" detail={batch.error} />
        </div>
      ) : batch.options && batch.draft ? (
        <BatchForm
          projectDir={batch.options.projectDir}
          draft={batch.draft}
          redo={Boolean(batch.redo)}
        />
      ) : (
        <div className="flex flex-1 items-center justify-center gap-2 text-fg-mute">
          <Icon name="circle-notch" spin size={16} /> Reading the refs
        </div>
      )}
    </Sheet>
  );
}
