import { Button, Icon, IconButton, InlineBanner } from '@maquette/ui';
import { KEYS } from '@/lib/keys';
import { PrepEditor } from '@/features/common/PrepEditor';
import { usePrepDialog } from './PrepDialog.hooks';
import type { PrepDialogProps } from './PrepDialog.types';

/**
 * Real size and fix-ups from a card's menu: what the Build scene step does
 * to the asset's model, on a copy, before the layout places it.
 */
export default function PrepDialog(props: PrepDialogProps) {
  const { asset, onClose } = props;
  const {
    ref,
    found,
    failed,
    value,
    setDraft,
    saving,
    changed,
    preview,
    onPreview,
    submit,
    applied,
  } = usePrepDialog(props);
  const version = found?.version ? `v${found.version}` : 'no model yet';

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
        aria-labelledby="prep-title"
        className="flex max-h-full w-[460px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onSubmit={e => void submit(e)}
      >
        <div className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="ruler" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="prep-title" className="truncate text-panel-title">
              Real size and fix-ups
            </h2>
            <p className="truncate text-meta text-fg-mute">
              {asset.name}, {version}. {applied}
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={onClose} />
        </div>
        <div className="min-h-0 overflow-y-auto px-5 py-4">
          {failed ? (
            <InlineBanner tone="danger" title="Could not read the real size" detail={failed} />
          ) : null}
          {found && value ? (
            <PrepEditor
              value={value}
              onChange={setDraft}
              measured={found.measured}
              fixed={found.fixed}
              suggestion={found.suggestion}
              defaults={found.defaults}
              preview={preview}
              onPreview={onPreview}
              previewBlocked={found.version === null ? 'The asset has no model yet' : null}
              disabled={saving}
            />
          ) : failed ? null : (
            <p className="text-fg-mute">Reading the saved real size and fix-ups.</p>
          )}
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" variant="primary" loading={saving} disabled={!found || saving}>
            {changed ? 'Save' : 'Done'}
          </Button>
        </div>
      </form>
    </div>
  );
}
