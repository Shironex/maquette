import { Icon, StatusChip, cn, type Status } from '@maquette/ui';
import type { PackItem, PackItemState } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { itemNote, itemNumbers } from '@/state/packs';
import { KIND_ICON } from '@/state/thumbs';

const CHIP: Record<PackItemState, { status: Status; label: string }> = {
  queued: { status: 'queued', label: 'Waiting' },
  running: { status: 'running', label: 'Running' },
  pass: { status: 'done', label: 'Pass' },
  kept: { status: 'done', label: 'Kept' },
  warn: { status: 'review', label: 'To look at' },
  fail: { status: 'failed', label: 'Failed' },
};

/** The file a card is named by: the OBJ, or the GLB without one. */
function fileName(item: PackItem, format: 'obj' | 'glb' | 'both'): string {
  return `${item.model.key}.${format === 'glb' ? 'glb' : 'obj'}`;
}

export interface PackCheckCardProps {
  item: PackItem;
  format: 'obj' | 'glb' | 'both';
}

/**
 * One model of a pack as the check sees it: the render of what was written
 * (its library picture until then), the file, its triangles and height,
 * and what the check found in a line.
 */
export default function PackCheckCard({ item, format }: PackCheckCardProps) {
  const url = fileUrl(item.picture ?? item.model.thumbnail);
  const chip = CHIP[item.state];
  const numbers = itemNumbers(item);
  const note = itemNote(item);
  return (
    <article
      aria-label={item.model.name}
      className={cn(
        'flex min-w-0 flex-col overflow-hidden rounded-card border bg-raised',
        item.state === 'fail'
          ? 'border-danger/50'
          : item.state === 'warn'
            ? 'border-clay-line'
            : 'border-line'
      )}
    >
      <div className="grid aspect-[4/3] place-items-center overflow-hidden bg-studio">
        {url ? (
          <img
            src={url}
            alt=""
            className={cn('size-full object-contain', item.state === 'queued' && 'opacity-45')}
          />
        ) : (
          <Icon name={KIND_ICON[item.model.kind]} size={28} className="text-fg-mute" />
        )}
      </div>
      <div className="flex min-w-0 flex-col gap-1 px-2.5 pt-2 pb-2.5">
        <b className="truncate font-mono text-[12px]" title={fileName(item, format)}>
          {fileName(item, format)}
        </b>
        {numbers ? (
          <span className="truncate font-mono text-meta text-fg-dim" title={numbers}>
            {numbers}
          </span>
        ) : null}
        <StatusChip status={chip.status} label={chip.label} className="self-start" />
        <span className="line-clamp-2 text-meta text-fg-mute" title={note}>
          {note}
        </span>
      </div>
    </article>
  );
}
