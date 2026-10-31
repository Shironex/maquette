import { Button, IconButton, cn } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import type { LibraryCollection, LibraryItem } from '@/ipc/client';
import { KEYS } from '@/lib/keys';
import { removeEntries } from '@/state/library-actions';
import { TagPopover } from '../TagPopover';
import { useLibraryBulkBar } from './LibraryBulkBar.hooks';

export interface LibraryBulkBarProps {
  /** The picked models, in the view's order. */
  picked: readonly LibraryItem[];
  /** How many models the view shows, for Select all. */
  shown: number;
  onSelectAll: () => void;
  onClear: () => void;
  /** Every tag in the library. */
  known: readonly string[];
  collections: readonly LibraryCollection[];
}

/**
 * The bar over the models while some are picked: how many, Select all,
 * and the actions that take the whole selection at once (Tag, Add to
 * collection, Use in project, Export pack, Remove). Nothing here spends
 * credits.
 */
export default function LibraryBulkBar(props: LibraryBulkBarProps) {
  const { picked, shown, onSelectAll, onClear, known, collections } = props;
  const ids = picked.map(i => i.entry.id);
  const n = picked.length;
  const all = n >= shown;
  const { open, close, toggle, collectionItems, projectItems, exportPack, renderModels } =
    useLibraryBulkBar(ids, collections);

  return (
    <div
      role="toolbar"
      aria-label="Selected models"
      className="relative flex flex-none items-center gap-2 rounded-card border border-clay-line bg-clay-soft py-1.5 pr-1.5 pl-3"
    >
      <span className="font-semibold whitespace-nowrap">{n} selected</span>
      <Button
        size="sm"
        variant="ghost"
        tooltip={all ? 'Clears the selection' : `Picks all ${shown} models in view`}
        onClick={all ? onClear : onSelectAll}
      >
        {all ? 'Select none' : `Select all ${shown}`}
      </Button>
      <span className="flex-1" />
      <div className="relative">
        <Button
          size="sm"
          icon="tag"
          data-tag-toggle
          aria-expanded={open === 'tag'}
          className={cn(open === 'tag' && 'bg-hover')}
          tooltip="Puts tags on the selected models or takes them off"
          onClick={() => toggle('tag')}
        >
          Tag
        </Button>
        {open === 'tag' ? (
          <TagPopover
            picked={picked}
            known={known}
            onClose={close}
            className="absolute top-full right-0 z-40 mt-1.5"
          />
        ) : null}
      </div>
      <div className="relative">
        <Button
          size="sm"
          icon="folder-plus"
          aria-expanded={open === 'collection'}
          tooltip="Puts the selected models in a collection; a model can be in several"
          onClick={() => toggle('collection')}
        >
          Add to collection
        </Button>
        {open === 'collection' ? (
          <Menu
            label="Add to collection"
            onClose={close}
            className="absolute top-full right-0 mt-1 w-[240px]"
            header={
              <p className="px-2.5 pt-1.5 pb-1 text-meta text-fg-mute">
                {collections.length
                  ? 'Or make one with New collection in the filters; it starts with these.'
                  : 'No collection yet. New collection in the filters makes one with these.'}
              </p>
            }
            items={collectionItems}
          />
        ) : null}
      </div>
      <div className="relative">
        <Button
          size="sm"
          icon="copy"
          aria-expanded={open === 'project'}
          tooltip="Copies the newest version of each into a project, at no credits"
          onClick={() => toggle('project')}
        >
          Use in project
        </Button>
        {open === 'project' ? (
          <Menu
            label="Use in project"
            onClose={close}
            className="absolute top-full right-0 mt-1 w-[240px]"
            items={projectItems}
          />
        ) : null}
      </div>
      <Button
        size="sm"
        icon="package"
        tooltip="Writes the selected models as one pack for another engine or a friend: OBJ or GLB, checked in Blender, zipped"
        onClick={exportPack}
      >
        Export pack
      </Button>
      <Button
        size="sm"
        icon="image"
        tooltip="Renders the selected models in headless Blender: clay, toon, blueprint, turntable, a contact sheet. No credits"
        onClick={renderModels}
      >
        Render
      </Button>
      <span aria-hidden="true" className="mx-0.5 h-5 w-px bg-line-strong" />
      <Button
        size="sm"
        variant="danger"
        icon="trash"
        tooltip="Takes the selected models out of the library; copies in projects stay, and Undo brings them back"
        onClick={() => void removeEntries(ids)}
      >
        Remove
      </Button>
      <IconButton
        icon="x"
        size="sm"
        label="Clear selection"
        shortcut={KEYS.escape}
        onClick={onClear}
      />
    </div>
  );
}
