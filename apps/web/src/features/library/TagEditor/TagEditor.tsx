import { Icon, TextField } from '@maquette/ui';
import type { LibraryEntry } from '@/ipc/client';
import { useTagEditor } from './TagEditor.hooks';

export interface TagEditorProps {
  entry: LibraryEntry;
}

/** One model's tags: each comes off with its x, and Enter in the field adds one. */
export default function TagEditor({ entry }: TagEditorProps) {
  const { typed, setTyped, busy, change, onKeyDown } = useTagEditor(entry);
  const chips = entry.tags.map(tag => (
    <li key={tag}>
      <button
        type="button"
        disabled={busy}
        aria-label={`Take the tag ${tag} off`}
        onClick={() => void change([], [tag])}
        className="inline-flex h-6 cursor-pointer items-center gap-1 rounded-pill border border-line px-2 font-mono text-[11.5px] text-fg-dim hover:border-line-strong hover:text-fg"
      >
        {tag}
        <Icon name="x" size={10} />
      </button>
    </li>
  ));
  return (
    <div className="flex flex-col gap-2">
      {entry.tags.length ? (
        <ul className="flex list-none flex-wrap gap-1" aria-label="Tags">
          {chips}
        </ul>
      ) : (
        <p className="text-meta text-amber">No tags yet.</p>
      )}
      <TextField
        icon="tag"
        aria-label={`Add a tag to ${entry.name}`}
        placeholder="Add a tag, then Enter"
        value={typed}
        maxLength={32}
        disabled={busy}
        onChange={e => setTyped(e.target.value)}
        onKeyDown={onKeyDown}
      />
    </div>
  );
}
