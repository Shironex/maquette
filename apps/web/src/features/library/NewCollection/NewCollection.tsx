import { Icon, TextField } from '@maquette/ui';
import { useNewCollection } from './NewCollection.hooks';

export interface NewCollectionProps {
  /** The models the new collection starts with: the selection, if any. */
  entries: readonly string[];
}

/**
 * New collection at the foot of the Collections filter: a click opens a
 * name field, Enter makes the collection (with the selected models in it),
 * Escape lets it go.
 */
export default function NewCollection({ entries }: NewCollectionProps) {
  const { naming, name, setName, open, onBlur, onKeyDown } = useNewCollection(entries);
  if (!naming) {
    return (
      <button
        type="button"
        onClick={open}
        className="flex h-8 w-full cursor-pointer items-center gap-2 rounded-control px-2.5 text-left text-fg-mute hover:bg-hover hover:text-fg"
      >
        <Icon name="plus" size={13} />
        <span className="flex-1 truncate">New collection</span>
        {entries.length ? (
          <span className="font-mono text-[11px]">{entries.length} picked</span>
        ) : null}
      </button>
    );
  }
  return (
    <TextField
      autoFocus
      aria-label="New collection name"
      placeholder="Name, then Enter"
      value={name}
      maxLength={60}
      onChange={e => setName(e.target.value)}
      onBlur={onBlur}
      onKeyDown={onKeyDown}
    />
  );
}
