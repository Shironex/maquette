import { Button, Icon, IconButton, TextField } from '@maquette/ui';
import { MAX_NAME_CHARS, useRenameAssetDialog } from './RenameAssetDialog.hooks';
import type { RenameAssetDialogProps } from './RenameAssetDialog.types';

/**
 * Rename from a card's menu. Only the name on the card changes: the asset's
 * id, files and versions stay, so refs and pipeline outputs keep working.
 */
export default function RenameAssetDialog(props: RenameAssetDialogProps) {
  const { onClose } = props;
  const { ref, name, rename, trimmed, tooLong, error, saving, submit } =
    useRenameAssetDialog(props);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <form
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="rename-title"
        className="flex w-[420px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onSubmit={e => void submit(e)}
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="file-text" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="rename-title" className="truncate text-panel-title">
              Rename asset
            </h2>
            <p className="truncate text-meta text-fg-mute">
              Its files and versions keep their names.
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" onClick={onClose} />
        </header>
        <div className="px-5 py-4">
          <TextField
            label="Name"
            autoFocus
            value={name}
            onFocus={e => e.currentTarget.select()}
            onChange={e => rename(e.target.value)}
            hint={`${trimmed.length} of ${MAX_NAME_CHARS} characters`}
            note={
              tooLong
                ? { tone: 'missing', text: `Keep the name to ${MAX_NAME_CHARS} characters.` }
                : error
                  ? { tone: 'missing', text: error }
                  : undefined
            }
          />
        </div>
        <footer className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            loading={saving}
            disabled={!trimmed || tooLong || saving}
          >
            Rename
          </Button>
        </footer>
      </form>
    </div>
  );
}
