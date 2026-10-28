import { Button, Checkbox, Icon, IconButton, PathField, Select, TextField } from '@maquette/ui';
import type { AssetKind } from '@/ipc/client';
import { KEYS } from '@/lib/keys';
import { fileName } from '@/state/built';
import { useAddBuiltDialog } from './AddBuiltDialog.hooks';
import type { AddBuiltDialogProps } from './AddBuiltDialog.types';

const KINDS: readonly { value: AssetKind; label: string }[] = [
  { value: 'prop', label: 'Prop' },
  { value: 'character', label: 'Character' },
  { value: 'scene', label: 'Scene' },
];

/**
 * Add built asset: a model a Blender script makes. The script and its
 * helpers are copied into the project's built/ folder and it builds
 * headless right away, with no credits and no approval.
 */
export default function AddBuiltDialog(props: AddBuiltDialogProps) {
  const { onClose } = props;
  const f = useAddBuiltDialog(props);
  const helperRows = f.helpers.map(path => (
    <li
      key={path}
      className="flex items-center gap-2 rounded-control border border-line px-2.5 py-1"
    >
      <Icon name="file-text" size={14} />
      <span className="min-w-0 flex-1 truncate font-mono text-meta" title={path}>
        {fileName(path)}
      </span>
      <IconButton
        icon="x"
        size="sm"
        label={`Remove ${fileName(path)}`}
        onClick={() => f.dropHelper(path)}
      />
    </li>
  ));

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <form
        ref={f.ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="built-title"
        className="flex w-[520px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onSubmit={e => void f.submit(e)}
      >
        <div className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="wrench" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="built-title" className="truncate text-panel-title">
              Add built asset
            </h2>
            <p className="text-meta text-fg-mute">
              A model a Blender script makes. It builds headless, and again when the script changes.
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={onClose} />
        </div>
        <div className="flex flex-col gap-3.5 px-5 py-4">
          <PathField
            label="Script"
            value={f.script}
            placeholder="~/room/scripts/laptop.py"
            onChange={f.changeScript}
            onBrowse={() => void f.pickScript()}
            browseLabel="Choose..."
            hint="A build() that returns the root object is used when the script has one."
          />
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[12px] leading-4 font-medium text-fg-dim">Helpers</span>
              <Button size="sm" icon="plus" onClick={() => void f.pickHelpers()}>
                Add files...
              </Button>
            </div>
            {f.helpers.length === 0 ? (
              <p className="text-meta text-fg-mute">
                Files the script imports or reads, such as a shared common.py. A helper already in
                the project with the same content is shared.
              </p>
            ) : (
              <ul aria-label="Helpers" className="flex flex-col gap-1">
                {helperRows}
              </ul>
            )}
          </div>
          <div className="grid grid-cols-[1fr_150px] gap-2">
            <TextField
              label="Name"
              value={f.name}
              placeholder="Laptop"
              onChange={e => f.rename(e.target.value)}
            />
            <Select label="Kind" options={KINDS} value={f.kind} onChange={f.setKind} />
          </div>
          <TextField
            label="Root object"
            mono
            value={f.root}
            placeholder="optional"
            onChange={e => f.setRoot(e.target.value)}
            hint="Exports this object and its children, when build() returns nothing. Several assets can share one script this way."
          />
          <Checkbox
            checked={f.replace}
            onChange={f.setReplace}
            label="Replace files in built/ that have other content"
          />
          {f.error ? (
            <p role="alert" className="text-meta text-danger">
              {f.error}
            </p>
          ) : null}
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" variant="primary" loading={f.saving} disabled={f.saving}>
            Add and build
          </Button>
        </div>
      </form>
    </div>
  );
}
