import { Field, Icon, Tooltip, cn } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { Menu } from '@/features/common/Menu';
import { inProject } from '@/state/lab';
import { RefChip } from './RefChip';
import { useReferences } from './References.hooks';

export interface ReferencesProps {
  view: LabView;
}

/** The reference chips: drop images, pick from refs/ or choose a file. */
export default function References({ view }: ReferencesProps) {
  const {
    refs,
    removeRef,
    menu,
    setMenu,
    over,
    zone,
    full,
    onDrop,
    onDragOver,
    onDragLeave,
    unused,
    choose,
    pick,
  } = useReferences(view);

  const chips = refs.map(ref => (
    <RefChip
      key={ref}
      url={fileUrl(inProject(view.projectDir, ref))}
      name={ref.slice(ref.lastIndexOf('/') + 1)}
      onRemove={() => removeRef(ref)}
    />
  ));
  const unusedButtons = unused.map(r => {
    const url = fileUrl(inProject(view.projectDir, r.file));
    return (
      <Tooltip key={r.file} label={`Adds ${r.name} as a reference`}>
        <button
          type="button"
          onClick={() => pick(r.file)}
          className="h-12 cursor-pointer overflow-hidden rounded-control bg-raised ring-clay hover:ring-2"
        >
          {url ? (
            <img src={url} alt={r.name} className="size-full object-cover" />
          ) : (
            <span className="text-meta">{r.name}</span>
          )}
        </button>
      </Tooltip>
    );
  });

  return (
    <Field id="lab-refs" label="References" hint="Drop images here to guide the look.">
      <div
        ref={zone}
        id="lab-refs"
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        onDrop={onDrop}
        className={cn(
          'relative -m-1 grid grid-cols-4 gap-2 rounded-card p-1 transition-colors duration-120',
          over && 'bg-clay-soft outline-[1.5px] outline-clay outline-dashed'
        )}
      >
        {chips}
        {full ? null : (
          <div>
            <Tooltip
              label="Add a reference image"
              detail="From refs/ or a file; dropping images here works too"
              disabled={menu}
            >
              <button
                type="button"
                aria-label="Add reference image"
                aria-haspopup="menu"
                aria-expanded={menu}
                onClick={() => setMenu(m => !m)}
                className="grid h-[62px] w-full cursor-pointer place-items-center rounded-card border border-dashed border-line-strong bg-transparent text-fg-mute hover:border-clay-line hover:text-fg"
              >
                <Icon name="plus" size={16} />
              </button>
            </Tooltip>
            {menu ? (
              <Menu
                label="Add a reference"
                onClose={() => setMenu(false)}
                className="absolute top-full right-0 left-0 z-30 mt-1"
                header={
                  unused.length > 0 ? (
                    <div className="flex flex-col gap-1.5 p-1.5 pb-1">
                      <span className="text-caption text-fg-mute uppercase">From refs/</span>
                      <div className="grid grid-cols-4 gap-1.5">{unusedButtons}</div>
                    </div>
                  ) : (
                    <p className="px-2.5 py-2 text-meta text-fg-mute">
                      Nothing in refs/ yet. Send picks there, or choose a file.
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
    </Field>
  );
}
