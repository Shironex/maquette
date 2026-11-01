import { Button, Icon, InlineBanner } from '@maquette/ui';
import type { FolderBudget, GameExport, GameTemplatePreview } from '@/ipc/client';
import { Sheet } from '@/features/common/Sheet';
import { TemplateCards } from '@/features/common/TemplateCards';
import { formatK } from '@/lib/format';
import { useGameTemplates, useTemplatesOpen } from './GameTemplates.hooks';

export interface GameTemplatesProps {
  project: string;
  busy: boolean;
}

const folderName = (folder: string) => (folder ? `${folder}/` : 'top level');
const numbers = (b: FolderBudget) => `${formatK(b.triangles)} triangles, ${b.texture} px`;

const textureWord = (e: GameExport) => (e.textures === 'keep' ? 'Keep' : e.textures.toUpperCase());

/** The export settings that differ, as rows of the change list. */
function exportRows(before: GameExport, after: GameExport) {
  const rows: { what: string; from: string; to: string }[] = [];
  if (before.textures !== after.textures) {
    rows.push({ what: 'Texture format', from: textureWord(before), to: textureWord(after) });
  }
  if (before.textureCap !== after.textureCap) {
    rows.push({
      what: 'Texture size cap',
      from: `${before.textureCap} px`,
      to: `${after.textureCap} px`,
    });
  }
  if (before.dropMaps !== after.dropMaps) {
    const word = (e: GameExport) => (e.dropMaps ? 'left out' : 'kept');
    rows.push({ what: 'Normal and metal maps', from: word(before), to: word(after) });
  }
  return rows;
}

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
  const exportChanges = exportRows(preview.exportBefore, preview.exportAfter).map(r => (
    <Change key={r.what} {...r} />
  ));
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
            {exportChanges}
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
        Other budget rows, any export setting not listed above, Bake light and the model folder
        stay. No file moves or goes away, and no assets are added. You can still edit every setting
        afterwards.
      </p>
    </section>
  );
}

function TemplatesSheet({ project, busy, onClose }: GameTemplatesProps & { onClose: () => void }) {
  const t = useGameTemplates(project, onClose);
  return (
    <Sheet
      id="game-templates"
      icon="squares-four"
      title="Use a template"
      meta="Export settings only"
      width={820}
      onClose={onClose}
    >
      <div className="flex min-h-0 flex-1">
        <div className="w-[300px] flex-none overflow-auto border-r border-line px-4 py-4">
          <TemplateCards
            templates={t.templates}
            picked={t.picked}
            onPick={template => t.pick(template.id)}
            columns="grid-cols-1"
            compact
          />
        </div>
        <div className="min-w-0 flex-1 overflow-auto px-5 py-4">
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
