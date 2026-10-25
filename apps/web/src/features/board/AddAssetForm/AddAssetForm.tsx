import { Button, Select, TextField } from '@maquette/ui';
import type { AssetKind } from '@/ipc/client';
import { STAGES, STAGE_LABEL } from '@/state/pipeline';
import { useAddAssetForm } from './AddAssetForm.hooks';
import type { AddAssetFormProps } from './AddAssetForm.types';

const KINDS: readonly { value: AssetKind; label: string }[] = [
  { value: 'prop', label: 'Prop' },
  { value: 'character', label: 'Character' },
  { value: 'scene', label: 'Scene' },
];

/** The small form under Add asset: a name, a kind and the starting column. */
export default function AddAssetForm({ project, onDone, onCancel }: AddAssetFormProps) {
  const { name, rename, kind, setKind, stage, setStage, error, saving, submit } = useAddAssetForm({
    project,
    onDone,
  });
  return (
    <form
      aria-label="Add asset"
      onSubmit={e => void submit(e)}
      onKeyDown={e => {
        if (e.key === 'Escape') onCancel();
      }}
      className="absolute top-full right-0 z-30 mt-2 flex w-[300px] flex-col gap-3 rounded-card border border-line-strong bg-panel p-4 shadow-window"
    >
      <TextField
        label="Name"
        autoFocus
        value={name}
        placeholder="Stone lantern"
        onChange={e => rename(e.target.value)}
        note={error ? { tone: 'missing', text: error } : undefined}
      />
      <div className="grid grid-cols-2 gap-2">
        <Select label="Kind" options={KINDS} value={kind} onChange={setKind} />
        <Select
          label="Column"
          options={STAGES.map(s => ({ value: s, label: STAGE_LABEL[s] }))}
          value={stage}
          onChange={setStage}
        />
      </div>
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" disabled={saving}>
          Add asset
        </Button>
      </div>
    </form>
  );
}
