import { Icon, cn } from '@maquette/ui';
import { useAvatarList } from './AvatarList.hooks';

/** The project's characters an avatar is built from. */
export default function AvatarList() {
  const { rows, asset, select } = useAvatarList();
  const items = rows.map(row => (
    <button
      key={row.id}
      type="button"
      aria-current={row.id === asset ? 'true' : undefined}
      onClick={() => select(row.id)}
      className={cn(
        'flex w-full cursor-pointer items-center gap-2.5 rounded-card border border-transparent p-2 text-left hover:bg-hover',
        row.id === asset && 'border-clay-line bg-clay-soft hover:bg-clay-soft'
      )}
    >
      <span className="grid size-11 flex-none place-items-center overflow-hidden rounded-[8px] bg-studio">
        {row.thumb ? (
          <img src={row.thumb} alt="" className="size-full object-cover" />
        ) : (
          <Icon name="person-arms-spread" size={18} className="text-fg-mute" />
        )}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-[13px] leading-[17px] font-semibold">{row.name}</span>
        <span className="truncate font-mono text-[11.5px] leading-4 text-fg-mute">{row.line}</span>
      </span>
    </button>
  ));
  return (
    <section
      aria-label="Characters"
      className="flex w-[272px] flex-none flex-col border-r border-line bg-panel"
    >
      <div className="px-4 pt-[18px] pb-3">
        <h2 className="text-panel-title">Avatar</h2>
        <p className="text-meta text-fg-mute">
          {rows.length} {rows.length === 1 ? 'character' : 'characters'}, exported as VRM 1.0
        </p>
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-2 pb-2">{items}</div>
    </section>
  );
}
