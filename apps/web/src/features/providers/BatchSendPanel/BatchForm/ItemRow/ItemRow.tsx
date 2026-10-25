import type { ReactNode } from 'react';
import { Icon, IconButton, SegmentedControl, cn } from '@maquette/ui';
import type { PresetVariant } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import type { BatchItemDraft } from '@/state/batch';
import { inProject } from '@/state/lab';
import { VARIANTS } from '../../../choices';

export interface ItemRowProps {
  item: BatchItemDraft;
  projectDir: string;
  onVariant: (variant: PresetVariant) => void;
  /** What the asset changes from the batch, or null when it follows it. */
  diff: string | null;
  open: boolean;
  onToggle: () => void;
  onClear: () => void;
  fields: ReactNode;
}

export default function ItemRow({
  item,
  projectDir,
  onVariant,
  diff,
  open,
  onToggle,
  onClear,
  fields,
}: ItemRowProps) {
  const thumb = fileUrl(item.thumbnail ? inProject(projectDir, item.thumbnail) : null);
  const n = item.images.length;
  const panel = `custom-${item.asset}`;
  const refs =
    n === 0
      ? 'No refs yet'
      : `${n} ${n === 1 ? 'image' : 'images'}: ${item.images.map(i => i.slice(i.lastIndexOf('/') + 1)).join(', ')}`;
  return (
    <li className="flex flex-col gap-2.5 border-t border-line py-2 first:border-t-0">
      <div className="flex items-center gap-3">
        <span className="grid size-10 flex-none place-items-center overflow-hidden rounded-control bg-studio">
          {thumb ? (
            <img src={thumb} alt="" className="size-full object-cover" />
          ) : (
            <Icon name="image" className="text-fg-mute" />
          )}
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="truncate font-medium">{item.name}</span>
            {diff !== null ? (
              <span className="flex-none rounded-control border border-clay-line bg-clay-soft px-1.5 text-[11px] font-medium text-fg">
                Custom
              </span>
            ) : null}
          </div>
          <div className={cn('truncate text-meta', n === 0 ? 'text-amber' : 'text-fg-mute')}>
            {refs}
          </div>
          {diff !== null ? (
            <div className="truncate text-meta text-fg-dim" title={diff}>
              {diff}
            </div>
          ) : null}
        </div>
        <SegmentedControl
          label={`${item.name} kind`}
          options={VARIANTS}
          value={item.variant}
          onChange={onVariant}
        />
        <IconButton
          icon="sliders-horizontal"
          size="sm"
          variant={open || diff !== null ? 'secondary' : 'ghost'}
          label={`Custom settings for ${item.name}`}
          aria-expanded={open}
          aria-controls={panel}
          onClick={onToggle}
        />
      </div>
      {open ? (
        <div
          id={panel}
          role="group"
          aria-label={`${item.name} settings`}
          className="flex flex-col gap-3 rounded-card border border-line bg-raised px-3 py-2.5"
        >
          <div className="flex items-center justify-between gap-3">
            <span className="text-meta text-fg-dim">
              Starts from the batch settings. Only the fields you change stay with this asset.
            </span>
            <button
              type="button"
              disabled={diff === null}
              className="flex-none cursor-pointer text-meta text-fg-mute underline-offset-2 hover:text-fg hover:underline disabled:cursor-default disabled:opacity-45 disabled:no-underline"
              onClick={onClear}
            >
              Use batch settings
            </button>
          </div>
          {fields}
        </div>
      ) : null}
    </li>
  );
}
