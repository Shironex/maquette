/**
 * One ref in the refs sheet: the image with its view chip. The chip opens
 * the views; picking one that is taken swaps the two refs.
 */
import { Icon, cn } from '@maquette/ui';
import type { AssetRef, RefView, RefWarning } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject } from '@/state/lab';
import { VIEW_LABEL } from '@/state/refs';
import { Menu } from '@/features/common/Menu';
import { nameOf, useViewTile } from './ViewTile.hooks';

export interface ViewTileProps {
  item: AssetRef;
  projectDir: string;
  /** Every ref of the asset, to say which view a pick swaps with. */
  refs: readonly AssetRef[];
  warnings?: readonly RefWarning[];
  disabled?: boolean;
  onTag: (view: RefView) => void;
  onClean: () => void;
  onRemove: () => void;
}

export default function ViewTile({
  item,
  projectDir,
  refs,
  warnings = [],
  disabled = false,
  onTag,
  onClean,
  onRemove,
}: ViewTileProps) {
  const { open, toggle, close, items } = useViewTile(item, refs, { onTag, onClean, onRemove });
  const url = fileUrl(inProject(projectDir, item.file));
  const name = nameOf(item.file);
  const flagged = warnings.length > 0;
  const flagTitle = warnings.map(w => w.title).join('\n');
  return (
    <li className="flex min-w-0 flex-col gap-1">
      <div className="relative aspect-square overflow-hidden rounded-[8px] border border-line bg-studio">
        {url ? (
          <img src={url} alt={name} className="size-full object-cover" draggable={false} />
        ) : (
          <Icon name="image" className="m-auto text-fg-mute" />
        )}
        {flagged ? (
          <span
            title={flagTitle}
            className="absolute right-1 bottom-1 inline-flex h-[18px] items-center gap-1 rounded-pill bg-panel/90 px-1.5 text-[10.5px] font-semibold text-amber"
          >
            <Icon name="warning" size={11} /> {warnings.length}
          </span>
        ) : null}
        <div className="absolute top-1 left-1">
          <button
            type="button"
            aria-haspopup="menu"
            aria-expanded={open}
            aria-label={`View of ${name}: ${VIEW_LABEL[item.view]}`}
            disabled={disabled}
            onClick={toggle}
            className={cn(
              'inline-flex h-[20px] cursor-pointer items-center gap-0.5 rounded-pill px-1.5 text-[11px] font-semibold disabled:cursor-not-allowed disabled:opacity-60',
              item.view === 'other' ? 'bg-panel/90 text-fg-dim' : 'bg-clay text-clay-ink'
            )}
          >
            {VIEW_LABEL[item.view]}
            <Icon name="caret-down" size={10} />
          </button>
          {open ? (
            <Menu
              label={`View of ${name}`}
              items={items}
              onClose={close}
              header={
                <p className="px-2.5 pt-1.5 pb-1 text-meta text-fg-mute">
                  A view holds one ref; picking a taken one swaps the two.
                </p>
              }
              className="absolute top-full left-0 mt-1 w-[230px]"
            />
          ) : null}
        </div>
      </div>
      <span className="truncate font-mono text-[11px] text-fg-mute" title={item.file}>
        {name}
      </span>
    </li>
  );
}
