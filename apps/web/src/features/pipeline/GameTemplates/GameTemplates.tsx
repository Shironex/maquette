import { Button, Icon, InlineBanner, cn } from '@maquette/ui';
import type { FolderBudget, GameTemplatePreview } from '@/ipc/client';
import { Sheet } from '@/features/common/Sheet';
import { formatK } from '@/lib/format';
import { useGameTemplates, useTemplatesOpen } from './GameTemplates.hooks';

export interface GameTemplatesProps {
  project: string;
  busy: boolean;
}

const folderName = (folder: string) => (folder ? `${folder}/` : 'top level');
const numbers = (b: FolderBudget) => `${formatK(b.triangles)} triangles, ${b.texture} px`;

function Change({ what, from, to }: { what: string; from: string; to: string }) {
  return (
    <li className="flex items-baseline gap-2 border-t border-line px-2.5 py-1.5 first:border-t-0">
      <span className="min-w-0 flex-1 truncate font-mono">{what}</span>
      <span className="text-fg-mute">{from}</span>
      <Icon name="arrow-right" size={12} className="flex-none self-center text-fg-mute" />
      <span className="font-medium">{to}</span>
    </li>
  );
}

function FileList({ title, files }: { title: string; files: string[] }) {
  if (!files.length) return null;
  const rows = files.map(file => (
    <li key={file} className="truncate" title={file}>
      {file}
    </li>
  ));
  return (
    <div className="flex flex-col gap-1">
      <h4 className="text-[12px] leading-4 font-medium text-fg-dim">{title}</h4>
      <ul className="m-0 flex list-none flex-col gap-0.5 p-0 font-mono text-meta text-fg-dim">
        {rows}
      </ul>
    </div>
  );
}

function Preview({ preview }: { preview: GameTemplatePreview }) {
  const { template: t, formatBefore, formatAfter, budgets } = preview;
  const formatChanges = formatBefore !== formatAfter;
  const budgetRows = budgets.map(b => (
    <Change
      key={b.folder}
      what={folderName(b.folder)}
      from={b.before ? numbers(b.before) : 'no budget'}
      to={numbers(b.after)}
    />
  ));
  const notes = preview.notes.map(note => <InlineBanner key={note} tone="warning" title={note} />);
  const newPaths = preview.newPaths.map(p => `${p.from} to ${p.to}`);
  return (
    <section aria-label="What will change" className="flex flex-col gap-3 text-meta">
      <p className="m-0 text-fg-dim">{t.why}</p>
      {preview.changes ? (
        <div className="flex flex-col gap-1">
          <h3 className="text-[12px] leading-4 font-medium text-fg-dim">What will change</h3>
          <ul className="m-0 flex list-none flex-col rounded-[7px] border border-line bg-raised p-0">
            {formatChanges ? (
              <Change what="New models" from={`.${formatBefore}`} to={`.${formatAfter}`} />
            ) : null}
            {budgetRows}
          </ul>
        </div>
      ) : (
        <p className="m-0 rounded-[8px] bg-raised px-2.5 py-2 text-fg-dim">
          This game already has these settings.
        </p>
      )}
      {preview.overBefore !== preview.overAfter ? (
        <p className="m-0 tabular-nums">
          Models over budget: {preview.overBefore} now, {preview.overAfter} after.
        </p>
      ) : null}
      {notes}
      <FileList title="In the game already, keep their path and format" files={preview.keptFiles} />
      <FileList title="Nothing in the game yet, export at a new path" files={newPaths} />
      <p className="m-0 text-fg-mute">
        Other budget rows, textures, the size cap, Bake light and the model folder stay. No file
        moves or goes away, and no assets are added. You can still edit every setting afterwards.
      </p>
    </section>
  );
}

function TemplatesSheet({ project, busy, onClose }: GameTemplatesProps & { onClose: () => void }) {
  const t = useGameTemplates(project, onClose);
  const cards = t.templates.map(template => {
    const on = template.id === t.picked;
    return (
      <button
        key={template.id}
        type="button"
        aria-pressed={on}
        onClick={() => t.pick(template.id)}
        className={cn(
          'cursor-pointer rounded-card border bg-raised px-3 py-2.5 text-left transition-colors duration-120',
          on ? 'border-clay shadow-[0_0_0_1px_var(--clay)]' : 'border-line hover:border-line-strong'
        )}
      >
        <span className="block text-[13px] font-semibold">{template.name}</span>
        <span className="mt-0.5 block text-meta text-fg-mute">{template.detail}</span>
      </button>
    );
  });
  return (
    <Sheet
      id="game-templates"
      icon="squares-four"
      title="Use a template"
      meta="Export settings only"
      onClose={onClose}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-5 py-4">
        <div className="grid grid-cols-2 gap-2" role="group" aria-label="Game templates">
          {cards}
        </div>
        {t.preview ? (
          <Preview preview={t.preview} />
        ) : (
          <p role="status" className="m-0 text-meta text-fg-mute">
            {t.reading
              ? 'Reading this game. If macOS asks to open its folder, answer it and this fills in.'
              : t.picked
                ? 'Could not read this game. Pick the template again to retry.'
                : 'Pick a template to see what it would change.'}
          </p>
        )}
      </div>
      <div className="flex justify-end gap-2 border-t border-line px-5 py-3">
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button
          variant="primary"
          loading={t.saving}
          disabled={!t.preview?.changes || busy}
          disabledReason={
            busy
              ? 'Wait for the run to finish'
              : t.preview
                ? 'This game already has these settings'
                : t.reading
                  ? 'Still reading this game'
                  : 'Pick a template first'
          }
          onClick={() => void t.apply()}
        >
          Apply template
        </Button>
      </div>
    </Sheet>
  );
}

/** Fills the game export settings from a template, after showing what changes. */
export default function GameTemplates({ project, busy }: GameTemplatesProps) {
  const { open, setOpen } = useTemplatesOpen();
  return (
    <>
      <Button size="sm" icon="squares-four" className="mt-2" onClick={() => setOpen(true)}>
        Use a template
      </Button>
      {open ? (
        <TemplatesSheet project={project} busy={busy} onClose={() => setOpen(false)} />
      ) : null}
    </>
  );
}
