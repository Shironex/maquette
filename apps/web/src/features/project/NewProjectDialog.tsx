import { useEffect, useRef, useState } from 'react';
import { Button, Icon, IconButton } from '@maquette/ui';
import { useAppStore } from '@/state/use-app-store';
import { emptyProjectFields, projectFieldsReady, type ProjectFields } from '@/state/onboarding';
import { useProjectsStore } from '@/state/use-projects-store';
import { ProjectForm } from './ProjectForm';
import { useProjectFieldActions } from './useProjectFieldActions';

/** New project and Import folder from the switcher: the onboarding form, compact. */
export function NewProjectDialog({ onClosed }: { onClosed?: () => void }) {
  const mode = useProjectsStore(s => s.dialog);
  if (!mode) return null;
  // Keyed on the mode it opened with, so each open starts from a clean form.
  return <Dialog key={mode} initialMode={mode} onClosed={onClosed} />;
}

function Dialog({
  initialMode,
  onClosed,
}: {
  initialMode: ProjectFields['mode'];
  onClosed?: () => void;
}) {
  const close = useProjectsStore(s => s.closeDialog);
  const create = useProjectsStore(s => s.create);
  const creating = useProjectsStore(s => s.creating);
  const current = useAppStore(s => s.workspace?.project?.name);
  const [fields, setFields] = useState<ProjectFields>(() => ({
    ...emptyProjectFields(),
    mode: initialMode,
  }));
  const update = (patch: Partial<ProjectFields>) => setFields(f => ({ ...f, ...patch }));
  const actions = useProjectFieldActions(fields, update);
  const sheet = useRef<HTMLFormElement>(null);
  const ready = projectFieldsReady(fields);
  const importing = fields.mode === 'import';

  const dismiss = () => {
    close();
    onClosed?.();
  };

  useEffect(() => {
    if (initialMode === 'import') sheet.current?.querySelector<HTMLElement>('input')?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        close();
        onClosed?.();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [close, onClosed, initialMode]);

  const submit = async () => {
    if (!ready) return;
    if (await create(fields)) onClosed?.();
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) dismiss();
      }}
    >
      <form
        ref={sheet}
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-project-title"
        className="flex max-h-full w-[560px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onSubmit={e => {
          e.preventDefault();
          void submit();
        }}
        onKeyDown={e => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void submit();
          }
        }}
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
          <IconButton icon="x" label="Close" onClick={dismiss} />
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
            title={ready ? undefined : importing ? 'Name it and pick a folder' : 'Name it first'}
          >
            {importing ? 'Import project' : 'Create project'}
          </Button>
        </footer>
      </form>
    </div>
  );
}
