import { Icon, Tooltip, cn } from '@maquette/ui';
import type { RefImage, RefView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatBytes } from '@/lib/format';
import { inProject } from '@/state/lab';
import { VIEW_NAMES } from '@/state/providers';
import { VIEW_LABEL } from '@/state/refs';

export interface ImageChoiceProps {
  image: RefImage;
  projectDir: string;
  order: number;
  /**
   * For an asset with view tags: the view the image goes as, null when it
   * stays home. Left out, the order names the view.
   */
  view?: RefView | null;
  /** How many ref warnings it has. */
  warned?: number;
  full: boolean;
  onToggle: () => void;
  onFront: () => void;
}

export default function ImageChoice({
  image,
  projectDir,
  order,
  view,
  warned = 0,
  full,
  onToggle,
  onFront,
}: ImageChoiceProps) {
  const url = fileUrl(inProject(projectDir, image.file));
  const on = order >= 0;
  // A tagged asset's views come from its tags, so there is no order to change.
  const canFront = on && order > 0 && view === undefined;
  const badge =
    view === undefined
      ? `${order + 1} ${VIEW_NAMES[order]}`
      : view
        ? VIEW_LABEL[view]
        : 'Other, stays home';
  return (
    <li className="group relative">
      <Tooltip
        label={`${on ? 'Leave out' : 'Send'} ${image.name}${image.bytes ? `, ${formatBytes(image.bytes)}` : ''}`}
      >
        <button
          type="button"
          aria-pressed={on}
          aria-label={`${image.name}${image.bytes ? `, ${formatBytes(image.bytes)}` : ''}`}
          disabled={!on && full}
          onClick={onToggle}
          className={cn(
            'relative block aspect-square w-full cursor-pointer overflow-hidden rounded-[8px] border bg-studio transition-colors duration-120 disabled:cursor-not-allowed disabled:opacity-40',
            on ? 'border-clay ring-2 ring-clay/35' : 'border-line hover:border-line-strong'
          )}
        >
          {url ? (
            <img src={url} alt="" className="size-full object-cover" draggable={false} />
          ) : (
            <Icon name="image" className="text-fg-mute" />
          )}
          {on ? (
            <span className="absolute top-1 left-1 inline-flex h-[18px] items-center gap-1 rounded-pill bg-clay px-1.5 text-[10.5px] font-semibold text-clay-ink">
              {badge}
            </span>
          ) : null}
          {warned > 0 ? (
            <span className="absolute right-1 bottom-1 inline-flex h-[18px] items-center gap-0.5 rounded-pill bg-panel/90 px-1.5 text-[10.5px] font-semibold text-amber">
              <Icon name="warning" size={11} /> {warned}
            </span>
          ) : null}
        </button>
      </Tooltip>
      <div className="mt-1 truncate font-mono text-[11px] text-fg-mute">{image.name}</div>
      {canFront ? (
        <Tooltip label="Makes this the front image the model is built from">
          <button
            type="button"
            onClick={onFront}
            className="absolute top-1 right-1 hidden h-[18px] cursor-pointer items-center rounded-pill bg-panel/90 px-1.5 text-[10.5px] font-medium text-fg group-focus-within:inline-flex group-hover:inline-flex"
          >
            Make front
          </button>
        </Tooltip>
      ) : null}
    </li>
  );
}
