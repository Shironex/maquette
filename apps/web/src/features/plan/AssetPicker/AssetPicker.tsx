import type { ReactNode } from 'react';
import { Checkbox, Icon, TextField } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import type { LinkPick, PickRow } from '@/state/plan-links';
import { useAssetPicker } from './AssetPicker.hooks';

export interface AssetPickerProps {
  /** What the canvas links already. */
  linked: { assets: readonly string[]; directions: readonly string[] };
  onAdd: (picked: LinkPick[]) => void;
  onClose: () => void;
}

/**
 * Brings assets of the Board and directions of the Lab onto the canvas.
 * The nodes keep only the id; everything else is read from the project.
 */
export default function AssetPicker({ linked, onAdd, onClose }: AssetPickerProps) {
  const v = useAssetPicker(linked, onAdd);
  const row = (r: PickRow) => {
    const src = v.src(r);
    return (
      <li key={r.key}>
        <Checkbox
          className="w-full rounded-[7px] px-2 py-1.5 hover:bg-hover"
          checked={v.isTicked(r)}
          onChange={on => v.tick(r, on)}
          label={
            <span className="flex min-w-0 flex-1 items-center gap-2.5">
              <span className="grid size-8 shrink-0 place-items-center overflow-hidden rounded-[6px] border border-line bg-studio text-fg-mute">
                {src ? (
                  <img src={src} alt="" className="size-full object-cover" />
                ) : (
                  <Icon name={r.kind === 'asset' ? 'cube' : 'flask'} size={15} />
                )}
              </span>
              <span className="flex min-w-0 flex-col">
                <span className="truncate text-[13px] text-fg">{r.name}</span>
                <span className="truncate text-meta text-fg-mute">
                  {r.meta}
                  {r.linked ? ' · on this canvas' : ''}
                </span>
              </span>
            </span>
          }
        />
      </li>
    );
  };
  const assets = v.assets.map(row);
  const directions = v.directions.map(row);
  const section = (title: string, items: ReactNode[], empty: string) => (
    <section aria-label={title} className="flex flex-col gap-1">
      <h3 className="px-2 text-[11px] font-medium tracking-[0.05em] text-fg-mute uppercase">
        {title}
      </h3>
      {items.length ? (
        <ul className="flex flex-col">{items}</ul>
      ) : (
        <p className="px-2 py-1 text-meta text-fg-mute">{empty}</p>
      )}
    </section>
  );
  return (
    <ChoiceDialog
      title="Link assets"
      onDismiss={onClose}
      choices={[
        { label: 'Cancel', onClick: onClose },
        {
          label: v.count > 1 ? `Add ${v.count}` : 'Add',
          variant: 'primary',
          disabled: v.count === 0,
          disabledReason: 'Tick an asset or a direction first',
          onClick: v.add,
        },
      ]}
    >
      <p>The canvas keeps only the link. Names, pictures and states come from the project.</p>
      <div className="mt-3 flex flex-col gap-3">
        <TextField
          id={v.search}
          label="Search"
          placeholder="Asset or direction name"
          value={v.query}
          onChange={e => v.setQuery(e.target.value)}
        />
        <div className="flex max-h-[300px] flex-col gap-3 overflow-y-auto">
          {section('On the Board', assets, v.query ? 'No asset by that name' : 'No assets yet')}
          {section(
            'In the Lab',
            directions,
            v.readingLab
              ? 'Reading the Lab...'
              : v.query
                ? 'No direction by that name'
                : 'No directions yet'
          )}
        </div>
      </div>
    </ChoiceDialog>
  );
}
