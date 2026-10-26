import { Button, Icon, IconButton } from '@maquette/ui';
import type { ProjectFields } from '@/state/onboarding';
import { ProjectForm } from '../../ProjectForm';
import { useNewProjectSheet } from './NewProjectSheet.hooks';
import { KEYS } from '@/lib/keys';

export interface NewProjectSheetProps {
  initialMode: ProjectFields['mode'];
  onClosed?: () => void;
}

export default function NewProjectSheet({ initialMode, onClosed }: NewProjectSheetProps) {
  const {
    fields,
    update,
    actions,
    sheet,
    ready,
    importing,
    game,
    creating,
    current,
    dismiss,
    onBackdropClick,
    onSubmit,
    onKeyDown,
  } = useNewProjectSheet(initialMode, onClosed);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={onBackdropClick}
    >
      <form
        ref={sheet}
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-project-title"
        className="flex max-h-full w-[560px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onSubmit={onSubmit}
        onKeyDown={onKeyDown}
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name={importing ? 'folder-open' : 'plus'} size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="new-project-title" className="truncate text-panel-title">
              {importing ? 'Import a folder' : 'New project'}
            </h2>
            <p className="truncate text-meta text-fg-mute">
              {current
                ? `It opens when it is ready. ${current} stays as it is.`
                : 'It opens when it is ready.'}
            </p>
          </div>
          <IconButton icon="x" label="Close" shortcut={KEYS.escape} onClick={dismiss} />
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          <ProjectForm
            value={fields}
            onChange={update}
            actions={actions}
            compact
            autoFocusName={initialMode === 'template'}
          />
        </div>
        <footer className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button variant="ghost" onClick={dismiss}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            iconRight="arrow-right"
            loading={creating}
            disabled={!ready || creating}
            disabledReason={
              game
                ? 'Name it and pick a game repo first'
                : importing
                  ? 'Name it and pick a folder first'
                  : 'Name it first'
            }
            shortcut={KEYS.submit}
          >
            {importing ? 'Import project' : 'Create project'}
          </Button>
        </footer>
      </form>
    </div>
  );
}
