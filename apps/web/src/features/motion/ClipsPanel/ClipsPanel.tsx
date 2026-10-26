import { Button, Icon, InlineBanner, Toggle, cn } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import { SectionLabel } from '@/features/common/SectionLabel';
import { BoneMapDialog } from '../BoneMapDialog';
import { ClipDetails } from '../ClipDetails';
import { OverBudget } from '../OverBudget';
import { RigCheck } from '../RigCheck';
import { useClipsPanel } from './ClipsPanel.hooks';

/** The selected model's clips, what ships, the clip's numbers and the rig check. */
export default function ClipsPanel() {
  const panel = useClipsPanel();
  const { model, rows } = panel;
  if (!model) return null;
  const clipRows = rows.map(row => (
    <div
      key={row.clip.name}
      className={cn(
        'flex items-center gap-2.5 rounded-card border border-line bg-raised px-2.5 py-2',
        row.clip.name === panel.selected && 'border-clay-line'
      )}
    >
      <button
        type="button"
        aria-pressed={row.clip.name === panel.selected}
        onClick={() => panel.selectClip(row.clip.name)}
        className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
      >
        <Icon name="film-strip" size={14} className="flex-none text-fg-mute" />
        {row.over ? (
          <span className="inline-flex flex-none items-center gap-1 rounded-pill bg-danger-soft px-1.5 text-[11.5px] leading-5 text-danger">
            <Icon name="x-circle" size={12} />
            Over budget
            <span className="sr-only">: {row.clip.name}</span>
          </span>
        ) : (
          <span
            className={cn(
              'min-w-0 truncate text-[13px] leading-[18px] font-semibold',
              !row.ship && 'text-fg-dim'
            )}
          >
            {row.clip.name}
          </span>
        )}
        <span className="ml-auto min-w-0 truncate font-mono text-[11.5px] text-fg-mute">
          {row.meta}
        </span>
        <span className="flex-none font-mono text-[11.5px] text-fg">{row.size}</span>
      </button>
      <Toggle
        aria-label={`Ship ${row.clip.name}`}
        checked={row.ship}
        disabled={!panel.supported}
        onChange={ship => panel.setShip(row.clip.name, ship)}
      />
    </div>
  ));
  return (
    <aside
      aria-label="Clips and rig"
      className="flex w-[344px] flex-none flex-col gap-3.5 overflow-y-auto border-l border-line bg-panel p-4"
    >
      <div className="relative flex items-center justify-between gap-2">
        <SectionLabel>Clips · {rows.length}</SectionLabel>
        <Button
          size="sm"
          icon="plus"
          iconRight="caret-down"
          aria-expanded={panel.menu}
          onClick={() => panel.setMenu(!panel.menu)}
        >
          Add clip
        </Button>
        {panel.menu ? (
          <Menu
            label="Add clip"
            className="absolute top-8 right-0 z-10 w-[260px]"
            onClose={() => panel.setMenu(false)}
            items={[
              {
                id: 'retarget',
                label: 'Retarget a library clip',
                icon: 'books',
                disabled: !model.rig,
                hint: model.rig ? undefined : 'Library clips need a rig',
                onSelect: panel.actions.retarget,
              },
              {
                id: 'inbox',
                label: 'From a .glb in the Inbox',
                icon: 'tray',
                onSelect: panel.actions.inbox,
              },
              {
                id: 'script',
                label: 'Bake with a Blender script',
                icon: 'terminal-window',
                onSelect: panel.actions.script,
              },
            ]}
          />
        ) : null}
      </div>
      {rows.length === 0 ? (
        <p className="-mt-1.5 text-meta text-fg-mute">
          Clips you add show here with their length and what they weigh on the site.
        </p>
      ) : (
        <div className="flex flex-col gap-1.5">{clipRows}</div>
      )}
      {rows.length > 0 && !panel.supported ? (
        <InlineBanner
          tone="warning"
          title="Every clip ships for now"
          detail="This project's Build scene script is older than the one that leaves clips out. Update the starter scripts in the Pipeline view."
          actions={
            <Button size="sm" icon="flow-arrow" onClick={panel.actions.pipeline}>
              Open Pipeline
            </Button>
          }
        />
      ) : null}
      {panel.over ? <OverBudget /> : rows.length > 0 ? <ClipDetails /> : null}
      <RigCheck />
      <BoneMapDialog />
    </aside>
  );
}
