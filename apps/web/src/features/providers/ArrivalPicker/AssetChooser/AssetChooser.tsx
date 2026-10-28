import { IconButton, TextField } from '@maquette/ui';
import { KEYS } from '@/lib/keys';
import { DownloadThumb } from '../PickerDialog/DownloadThumb';
import { choiceDomId, useAssetChooser } from './AssetChooser.hooks';
import { ChoiceRow } from './ChoiceRow';

export interface AssetChooserProps {
  /** The download being attached. */
  arrival: string;
}

/**
 * Attach a download to an asset: a searchable list of the open project's
 * assets, the ones a By hand send waits on first, each with its ref
 * picture, name and state. Keyboard first, for several files in a row.
 */
export default function AssetChooser({ arrival: id }: AssetChooserProps) {
  const c = useAssetChooser(id);
  if (!c.arrival) return null;
  const rows = c.choices.map(choice => (
    <ChoiceRow
      key={choice.asset}
      id={choiceDomId(c.listId, choice.asset)}
      choice={choice}
      active={c.active?.asset === choice.asset}
      suggested={c.arrival?.asset === choice.asset}
      onPick={() => c.pick(choice)}
    />
  ));
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.35)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) c.close();
      }}
    >
      <div
        ref={c.ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="chooser-title"
        className="flex max-h-full w-[480px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onKeyDown={c.onDialogKey}
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <DownloadThumb arrival={c.arrival} />
          <div className="min-w-0 flex-1">
            <h2 id="chooser-title" className="truncate text-panel-title">
              Attach to an asset
            </h2>
            <p className="truncate font-mono text-[12px] text-fg-mute" title={c.arrival.path}>
              {c.arrival.name}
            </p>
          </div>
          <IconButton icon="x" label="Close" shortcut={KEYS.escape} onClick={c.close} />
        </header>
        <div className="flex min-h-0 flex-1 flex-col gap-2 px-5 py-4">
          <TextField
            role="combobox"
            aria-label={`Search the assets of ${c.project?.name ?? 'the project'}`}
            aria-expanded="true"
            aria-controls={c.listId}
            aria-autocomplete="list"
            aria-activedescendant={c.active ? choiceDomId(c.listId, c.active.asset) : undefined}
            placeholder="Search assets"
            icon="magnifying-glass"
            autoComplete="off"
            value={c.query}
            onChange={e => c.search(e.target.value)}
            onKeyDown={c.onInputKey}
          />
          <div
            id={c.listId}
            role="listbox"
            aria-label="Assets"
            aria-busy={c.busy}
            className="max-h-[340px] min-h-0 overflow-y-auto overscroll-contain rounded-card border border-line bg-raised"
          >
            {rows.length === 0 ? (
              <p className="px-3 py-4 text-center text-meta text-fg-mute">
                {c.query.trim()
                  ? `No asset matches “${c.query.trim()}”.`
                  : 'This project has no assets yet.'}
              </p>
            ) : (
              rows
            )}
          </div>
        </div>
        <footer className="flex items-center gap-2 border-t border-line px-5 py-3 text-meta text-fg-mute">
          <span className="min-w-0 flex-1">Enter sends it to the Inbox for that asset.</span>
          {c.more > 0 ? <span>{c.more} more to attach next</span> : null}
        </footer>
      </div>
    </div>
  );
}
