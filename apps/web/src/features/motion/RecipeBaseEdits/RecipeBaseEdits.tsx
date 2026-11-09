import { Button, Icon, InlineBanner, type IconName } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import { ClipList } from './ClipList';
import { MirrorForm } from './MirrorForm';
import { TimeForm } from './TimeForm';
import { TrimForm } from './TrimForm';
import { useRecipeBaseEdits, type BaseEditItem } from './RecipeBaseEdits.hooks';

const ICONS: Record<BaseEditItem['what'], IconName> = {
  clips: 'film-strip',
  trim: 'frame-corners',
  time: 'gauge',
  mirror: 'arrows-left-right',
  layer: 'sliders-horizontal',
  blank: 'map-pin',
};

function Form({ item }: { item: BaseEditItem }) {
  if (item.what === 'clips') return <ClipList />;
  if (item.what === 'trim') return <TrimForm />;
  if (item.what === 'time') return <TimeForm index={Number(item.id)} />;
  if (item.what === 'mirror') return <MirrorForm />;
  return null;
}

/**
 * The base edits that shape the clip the changes sit on: the clips joined,
 * their trims, each retime and hold, and the mirror, with Add. One opens to
 * edit at a time.
 */
export default function RecipeBaseEdits() {
  const v = useRecipeBaseEdits();
  const rows = v.items.map(item => {
    const open = v.isOpen(item);
    return (
      <li key={item.key} className="rounded-card border border-line bg-raised">
        <button
          type="button"
          aria-expanded={open}
          onClick={() => v.toggle(item)}
          className="flex w-full items-center gap-2.5 px-3 py-2 text-left"
        >
          <Icon name={ICONS[item.what]} size={16} className="flex-none text-fg-dim" />
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[13px] font-medium">{item.title}</span>
            <span className="block truncate text-meta text-fg-mute">{item.detail}</span>
            {item.problem && !open ? (
              <span className="block truncate text-meta text-danger">{item.problem}</span>
            ) : null}
          </span>
          <Icon name={open ? 'caret-up-down' : 'caret-down'} size={14} className="text-fg-mute" />
        </button>
        {open ? <Form item={item} /> : null}
      </li>
    );
  });
  const stale = v.stale.map(name => (
    <InlineBanner
      key={name}
      tone="warning"
      title={`${name} changed in the library`}
      detail="Its keys have another hash than this recipe knows. The last bake used the old ones. Bake again to use them."
    />
  ));
  return (
    <section aria-label="Base edits" className="flex flex-col gap-2">
      <div className="flex items-end gap-2">
        <h3 className="flex-1 text-[13px] font-semibold">Base edits</h3>
        <span className="relative">
          <Button
            size="sm"
            icon="plus"
            aria-haspopup="menu"
            aria-expanded={v.adding}
            onClick={() => v.setAdding(!v.adding)}
          >
            Add
          </Button>
          {v.adding ? (
            <Menu
              label="Add a base edit"
              items={v.menu}
              onClose={() => v.setAdding(false)}
              className="absolute top-[calc(100%+2px)] right-0 z-30 w-[260px]"
            />
          ) : null}
        </span>
      </div>
      {stale}
      <ul aria-label="Recipe base edits" className="flex list-none flex-col gap-1.5">
        {rows}
      </ul>
      {v.mixamoNote ? <p className="text-meta text-fg-mute">{v.mixamoNote}</p> : null}
      <p className="text-meta text-fg-mute">
        Base edits come first. They shape the clip the changes sit on, and the changes keep their
        place on the motion when you trim or retime.
      </p>
    </section>
  );
}
