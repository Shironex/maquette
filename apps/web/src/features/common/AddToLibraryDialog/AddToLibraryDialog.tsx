import { Button, Select, TextField } from '@maquette/ui';
import type { AssetRef } from '@/state/use-library-store';
import { LibraryDialogFrame } from '../LibraryDialogFrame';
import { useAddToLibraryDialog } from './AddToLibraryDialog.hooks';

export interface AddToLibraryDialogProps {
  target: AssetRef | 'pick';
}

/**
 * Add to library: copies an asset's newest model, picture, refs and fix-ups
 * into the workspace library. A newer model of an asset already there adds
 * a version to its entry; the same model adds nothing.
 */
export default function AddToLibraryDialog({ target }: AddToLibraryDialogProps) {
  const f = useAddToLibraryDialog(target);
  const projectOptions = f.projects.map(p => ({ value: p.id, label: p.name }));
  const assetOptions = f.withModel.map(a => ({ value: a.id, label: a.name }));
  const help = f.entry
    ? `It came from the library's ${f.entry.name}. A newer model adds a version there.`
    : 'Any project in this workspace can then use a copy of it, without spending credits again.';

  return (
    <LibraryDialogFrame
      id="library-add-title"
      title={f.asset && !f.picking ? `Add ${f.asset.name} to the library` : 'Add to library'}
      help={help}
      onClose={f.close}
      onSubmit={() => void f.submit()}
      footer={
        <>
          <Button variant="ghost" onClick={f.close}>
            Cancel
          </Button>
          <Button type="submit" variant="primary" loading={f.busy} disabled={f.busy || !f.asset}>
            {f.entry ? 'Add as a new version' : 'Add to library'}
          </Button>
        </>
      }
    >
      {f.picking ? (
        <div className="grid grid-cols-2 gap-2">
          <Select
            label="Project"
            options={projectOptions}
            value={f.projectId}
            onChange={f.pickProject}
          />
          {assetOptions.length ? (
            <Select label="Asset" options={assetOptions} value={f.assetId} onChange={f.pickAsset} />
          ) : (
            <p className="self-end pb-2 text-meta text-fg-mute">No asset here has a model yet.</p>
          )}
        </div>
      ) : null}
      <TextField
        label="Name in the library"
        value={f.name}
        placeholder="Desk lamp"
        onChange={e => f.setName(e.target.value)}
      />
      <TextField
        label="Tags"
        value={f.tags}
        placeholder="props, desk"
        onChange={e => f.setTags(e.target.value)}
        hint="Comma separated. The library's filters list them."
      />
      {f.latest ? (
        <p className="text-meta text-fg-mute">
          Copies v{f.latest}, its picture, refs, real size, fix-ups and material tweaks. The project
          keeps its own copy.
        </p>
      ) : null}
      {f.error ? (
        <p role="alert" className="text-meta text-danger">
          {f.error}
        </p>
      ) : null}
    </LibraryDialogFrame>
  );
}
