import { Icon, cn } from '@maquette/ui';
import type { RefImage } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatBytes } from '@/lib/format';
import { inProject } from '@/state/lab';
import { VIEW_NAMES } from '@/state/providers';

export interface ImageChoiceProps {
  image: RefImage;
  projectDir: string;
  order: number;
  full: boolean;
  onToggle: () => void;
  onFront: () => void;
}

export default function ImageChoice({
  image,
  projectDir,
  order,
  full,
  onToggle,
  onFront,
}: ImageChoiceProps) {
  const url = fileUrl(inProject(projectDir, image.file));
  const on = order >= 0;
  return (
    <li className="group relative">
      <button
        type="button"
        aria-pressed={on}
        disabled={!on && full}
        onClick={onToggle}
        title={`${image.name}${image.bytes ? `, ${formatBytes(image.bytes)}` : ''}`}
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
            {order + 1} {VIEW_NAMES[order]}
          </span>
        ) : null}
      </button>
      <div className="mt-1 truncate font-mono text-[11px] text-fg-mute">{image.name}</div>
      {on && order > 0 ? (
        <button
          type="button"
          onClick={onFront}
          className="absolute top-1 right-1 hidden h-[18px] cursor-pointer items-center rounded-pill bg-panel/90 px-1.5 text-[10.5px] font-medium text-fg group-focus-within:inline-flex group-hover:inline-flex"
        >
          Make front
        </button>
      ) : null}
    </li>
  );
}
