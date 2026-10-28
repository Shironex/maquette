import { Icon, IconButton, Tooltip } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { Menu } from '@/features/common/Menu';
import { MAX_STYLE_IMAGES } from '@/state/lab-style';
import { useStyleImages } from './StyleImages.hooks';

export interface StyleImagesProps {
  view: LabView;
  /** The draft's images: board files, project files or absolute paths. */
  images: readonly string[];
  onAdd: (file: string) => void;
  onRemove: (file: string) => void;
}

/** The board's images: each with Remove, and Add from the project or a file. */
export default function StyleImages({ view, images, onAdd, onRemove }: StyleImagesProps) {
  const { shown, offered, full, menu, setMenu, pick, choose } = useStyleImages(view, images, onAdd);
  const tiles = shown.map(image => (
    <figure
      key={image.file}
      className="group relative m-0 aspect-[4/3] overflow-hidden rounded-card bg-raised"
      title={image.name}
    >
      {image.url ? (
        <img src={image.url} alt={image.name} className="block size-full object-cover" />
      ) : (
        <span className="grid size-full place-items-center px-1 text-center text-meta text-fg-mute">
          {image.name}
        </span>
      )}
      <IconButton
        icon="x"
        label={`Remove ${image.name}`}
        size="sm"
        variant="secondary"
        onClick={() => onRemove(image.file)}
        className="absolute top-1 right-1 size-6 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100"
      />
    </figure>
  ));
  const offers = offered.map(o => (
    <Tooltip key={o.file} label={`Adds ${o.name} to the board`}>
      <button
        type="button"
        aria-label={`Add ${o.name}`}
        onClick={() => pick(o.file)}
        className="h-12 cursor-pointer overflow-hidden rounded-control bg-raised ring-clay hover:ring-2"
      >
        {o.url ? (
          <img src={o.url} alt="" className="size-full object-cover" />
        ) : (
          <span className="text-meta">{o.name}</span>
        )}
      </button>
    </Tooltip>
  ));

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-baseline justify-between">
        <span className="font-medium">Images</span>
        <span className="text-meta text-fg-mute tabular-nums">
          {images.length} of {MAX_STYLE_IMAGES}
        </span>
      </div>
      <div className="relative grid grid-cols-4 gap-2">
        {tiles}
        {full ? null : (
          <div>
            <Tooltip
              label="Add a style image"
              detail="A concept or ref of this project, or a file; it is copied into style/"
              disabled={menu}
            >
              <button
                type="button"
                aria-label="Add style image"
                aria-haspopup="menu"
                aria-expanded={menu}
                onClick={() => setMenu(m => !m)}
                className="grid aspect-[4/3] w-full cursor-pointer place-items-center rounded-card border border-dashed border-line-strong bg-transparent text-fg-mute hover:border-clay-line hover:text-fg"
              >
                <Icon name="plus" size={16} />
              </button>
            </Tooltip>
            {menu ? (
              <Menu
                label="Add a style image"
                onClose={() => setMenu(false)}
                className="absolute top-full right-0 left-0 z-30 mt-1"
                header={
                  offered.length > 0 ? (
                    <div className="flex flex-col gap-1.5 p-1.5 pb-1">
                      <span className="text-caption text-fg-mute uppercase">From this project</span>
                      <div className="grid grid-cols-4 gap-1.5">{offers}</div>
                    </div>
                  ) : (
                    <p className="px-2.5 py-2 text-meta text-fg-mute">
                      Star or pick concepts to offer them here, or choose a file.
                    </p>
                  )
                }
                items={[
                  {
                    id: 'file',
                    label: 'Choose a file',
                    icon: 'folder-open',
                    onSelect: () => void choose(),
                  },
                ]}
              />
            ) : null}
          </div>
        )}
      </div>
      <p className="text-meta text-fg-mute">
        A few images that show the look. Codex takes their materials, light and colour, not their
        subjects.
      </p>
    </div>
  );
}
