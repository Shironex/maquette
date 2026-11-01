import { Button, Icon, InlineBanner, TextField, Toggle, cn } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import { SectionLabel } from '@/features/common/SectionLabel';
import { ClipDetails } from '../ClipDetails';
import { OverBudget } from '../OverBudget';
import { RigCheck } from '../RigCheck';
import { SEARCH_FROM, useClipsPanel } from './ClipsPanel.hooks';

/** The selected model's clips, what ships, the clip's numbers and the rig check. */
export default function ClipsPanel() {
  const panel = useClipsPanel();
  const { model, rows, total, groups } = panel;
  if (!model) return null;
  const clipRow = (row: (typeof rows)[number]) => (
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
        title={row.clip.name}
        className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
      >
        <Icon name="film-strip" size={14} className="flex-none self-start pt-0.5 text-fg-mute" />
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span
            className={cn(
              'line-clamp-2 break-all text-[13px] leading-[18px] font-semibold',
              !row.ship && 'text-fg-dim'
            )}
          >
            {row.clip.name}
          </span>
          <span className="flex items-center gap-2 font-mono text-[11.5px] leading-4">
            {row.over ? (
              <span
                title="Leaving it off brings the scene within its budget"
                className="inline-flex flex-none items-center gap-1 rounded-pill bg-danger-soft px-1.5 font-sans leading-4 text-danger"
              >
                <Icon name="x-circle" size={12} />
                Over
              </span>
            ) : (
              <span className="text-fg-mute">{row.meta}</span>
            )}
            <span className="ml-auto text-fg">{row.size}</span>
          </span>
        </span>
      </button>
      <Toggle
        aria-label={`Ship ${row.clip.name}`}
        checked={row.ship}
        disabled={!panel.supported}
        onChange={ship => panel.setShip(row.clip.name, ship)}
      />
    </div>
  );
  const ownRows = groups?.own.map(clipRow);
  const libraryRows = groups?.fromLibrary.map(clipRow);
  const allRows = rows.map(clipRow);
  return (
    <aside
      aria-label="Clips and rig"
      className="flex w-[300px] flex-none max-[1023px]:w-[240px] min-[1280px]:w-[344px] flex-col gap-3.5 overflow-y-auto border-l border-line bg-panel p-4"
    >
      <div className="relative flex items-center justify-between gap-2">
        <SectionLabel>
          Clips · {rows.length === total ? total : `${rows.length} of ${total}`}
        </SectionLabel>
        <Button
          size="sm"
          icon="plus"
          iconRight="caret-down"
          aria-expanded={panel.menu}
          tooltip="Add a clip from the motion library, a file or a Blender script"
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
                label: 'Add from library',
                icon: 'books',
                disabled: !model.rig,
                hint: model.rig ? undefined : 'Library clips need a rig',
                onSelect: panel.actions.retarget,
              },
              {
                id: 'save',
                label: 'Save clips to library',
                icon: 'download-simple',
                disabled: !model.rig || model.clips.length === 0,
                hint: !model.rig
                  ? 'Only a rigged model has clips to keep'
                  : model.clips.length === 0
                    ? 'This model has no clips yet'
                    : undefined,
                onSelect: panel.actions.save,
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
      {total >= SEARCH_FROM ? (
        <TextField
          type="search"
          icon="magnifying-glass"
          aria-label="Search clips"
          placeholder="Search clips by name"
          value={panel.query}
          maxLength={120}
          onChange={e => panel.setQuery(e.target.value)}
        />
      ) : null}
      {total === 0 ? (
        <p className="-mt-1.5 text-meta text-fg-mute">
          Clips you add show here with their length and what they weigh{' '}
          {panel.game ? 'in the model' : 'on the site'}.
        </p>
      ) : rows.length === 0 ? (
        <p className="-mt-1.5 text-meta text-fg-mute">No clip name has all of those words.</p>
      ) : (
        <div className="flex flex-col gap-1.5">
          {groups ? (
            <>
              <p className="text-meta text-fg-mute">In the model file · {groups.own.length}</p>
              {ownRows}
              <p className="mt-1.5 flex items-center justify-between text-meta text-fg-mute">
                From the library · {groups.fromLibrary.length}
                {model.version === groups.newest ? (
                  <span className="rounded-pill bg-sage-soft px-1.5 text-sage">
                    new in v{groups.newest}
                  </span>
                ) : null}
              </p>
              {libraryRows}
            </>
          ) : (
            allRows
          )}
        </div>
      )}
      {panel.leftOut.length ? (
        <p className="text-meta text-fg-mute">
          {panel.leftOut.join(', ')} failed in review and{' '}
          {panel.leftOut.length === 1 ? 'was' : 'were'} left out. Fix{' '}
          {panel.leftOut.length === 1 ? 'it' : 'them'} in the library, then add{' '}
          {panel.leftOut.length === 1 ? 'it' : 'them'} again.
        </p>
      ) : null}
      {panel.gameNote ? (
        <p className="text-meta text-fg-mute">
          Game export keeps the clips switched on in the model file; one switched off stays out of
          it from the next run. Clips count toward no budget here; the model&apos;s folder budget
          checks triangles and textures.
        </p>
      ) : null}
      {panel.oldStarters ? (
        <InlineBanner
          tone="warning"
          title="Every clip ships for now"
          detail={`This project's ${panel.game ? 'Export' : 'Build scene'} script is older than the one that leaves clips out. Update the starter scripts in the Pipeline view.`}
          actions={
            <Button
              size="sm"
              icon="flow-arrow"
              tooltip="Goes to the Pipeline, where the starter scripts are updated"
              onClick={panel.actions.pipeline}
            >
              Open Pipeline
            </Button>
          }
        />
      ) : null}
      {panel.over ? <OverBudget /> : total > 0 ? <ClipDetails /> : null}
      <RigCheck />
    </aside>
  );
}
