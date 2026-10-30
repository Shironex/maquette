import { Button, EmptyState, Icon, InlineBanner } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { SheetButton } from '../SheetButton';
import { SpringRow } from './SpringRow';
import { SpringSheets } from './SpringSheets';
import { useSpringsPanel } from './SpringsPanel.hooks';

const HELP =
  'Spring bones let hair, cloth and charms swing. Blender proposes the parts that hang off the body; you pick the ones that should swing.';

/**
 * Springs: the parts that could swing, found on the model the VRM is
 * written from and drawn on a sheet in their colours; the person confirms
 * the ones that should, tunes their joints and swing, makes their chains,
 * and approves the chains on a sway test before the VRM carries them.
 */
export default function SpringsPanel() {
  const p = useSpringsPanel();
  if (!p.view) return null;
  if (!p.ready) {
    return (
      <Panel id="avatar-springs" title="Springs" help={HELP}>
        <EmptyState
          icon="person-arms-spread"
          title="The rest pose comes first"
          body="The springs are found on the rest pose the VRM is written from. Build on the Export tab and approve the new rest pose, then come back."
          action={
            <Button icon="arrow-left" tooltip="Opens the Export tab" onClick={p.actions.toExport}>
              Go to Export
            </Button>
          }
        />
      </Panel>
    );
  }
  const find = (
    <Button
      variant={p.found ? 'secondary' : 'primary'}
      size={p.found ? 'sm' : 'md'}
      icon="magnifying-glass"
      loading={p.status.finding}
      disabled={p.busy}
      tooltip="Looks for parts hanging off the body, headless in Blender. Changes nothing."
      onClick={p.actions.find}
    >
      {p.found ? 'Look again' : 'Find dangling parts'}
    </Button>
  );
  const rows = p.candidates.map(c => (
    <SpringRow
      key={c.id}
      candidate={c}
      spring={p.confirmed.find(s => s.id === c.id) ?? null}
      disabled={p.busy}
      onConfirm={on => p.actions.confirm(c, on)}
      onEdit={change => p.actions.edit(c.id, change)}
    />
  ));
  return (
    <Panel id="avatar-springs" title="Springs" help={HELP} actions={p.found ? find : null}>
      {!p.found ? (
        <EmptyState
          icon="magnifying-glass"
          title="Nobody looked yet"
          body="Blender reads the rest pose and proposes the parts that hang off the body: a ponytail, a cape's panels, a charm. Nothing swings until you confirm it."
          action={find}
        />
      ) : p.candidates.length === 0 ? (
        <InlineBanner
          tone="info"
          title="No dangling parts found"
          detail="Nothing hangs off the body far enough to swing. The VRM is written without springs."
        />
      ) : (
        <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)]">
          <div className="flex flex-col gap-2">
            <SheetButton
              src={p.sheets.candidates}
              label="Parts found, each in its colour"
              onOpen={() => p.zoom.open('candidates')}
            />
          </div>
          <div className="flex flex-col gap-3">
            <ul
              aria-label="Parts that could swing"
              className="m-0 flex list-none flex-col gap-2 p-0"
            >
              {rows}
            </ul>
            <p className="text-meta text-fg-mute">
              Confirm only parts that should swing: a stiff brim or a sewn on patch looks wrong
              moving. The VRM gets a spring for each one after you approve its chain.
            </p>
            <Button
              variant="primary"
              icon="bone"
              className="self-start"
              loading={p.status.building}
              disabled={p.busy || p.makeBlocked !== null}
              tooltip={
                p.makeBlocked ??
                'Saves, adds a chain of joints to each confirmed part headless in Blender, then waits for your yes'
              }
              onClick={p.actions.make}
            >
              Make chains
            </Button>
          </div>
        </div>
      )}
      {p.chained ? (
        <section aria-labelledby="avatar-chains-title" className="flex flex-col gap-3">
          <h3 id="avatar-chains-title" className="text-emphasis font-semibold">
            Chains
          </h3>
          {p.state === 'pending' ? (
            <section
              aria-label="Spring chains approval"
              className="flex flex-col gap-3 rounded-card border border-amber/40 bg-amber-soft/40 p-3.5"
            >
              <div className="flex items-start gap-3">
                <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
                  <Icon name="person-arms-spread" size={17} />
                </span>
                <div className="min-w-0 flex-1">
                  <h4 className="text-emphasis leading-5 font-semibold">Approve the chains?</h4>
                  <p className="mt-0.5 text-meta text-fg-mute">
                    On the sway test each part should bend smoothly, stay on where it hangs from and
                    keep its shape. It swings through the body there; colliders keep it out in a VRM
                    app.
                  </p>
                </div>
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  variant="secondary"
                  disabled={p.busy}
                  tooltip="Writes the VRM without springs"
                  onClick={p.actions.deny}
                >
                  Deny
                </Button>
                <Button
                  variant="primary"
                  icon="check"
                  loading={p.status.deciding}
                  disabled={p.busy}
                  tooltip="Approves these chains and writes the VRM with springs"
                  onClick={p.actions.approve}
                >
                  Approve and write
                </Button>
              </div>
            </section>
          ) : p.state === 'denied' ? (
            <InlineBanner
              tone="warning"
              title="You turned these chains down"
              detail="The VRM is written without springs. Change the parts or their joints and make the chains again."
            />
          ) : (
            <p className="flex items-center gap-1.5 text-meta text-sage">
              <Icon name="check-circle" size={15} />
              Approved {p.approvedAgo}. The VRM carries these springs.
            </p>
          )}
          <p className="text-meta text-fg-dim">{p.summary}</p>
          <div className="flex gap-4">
            <SheetButton
              src={p.sheets.before}
              label="Chains"
              onOpen={() => p.zoom.open('before')}
            />
            <SheetButton
              src={p.sheets.after}
              label="Sway test"
              onOpen={() => p.zoom.open('after')}
            />
          </div>
        </section>
      ) : null}
      {p.zoom.kind ? (
        <SpringSheets sheets={p.sheets} start={p.zoom.kind} onClose={p.zoom.close} />
      ) : null}
    </Panel>
  );
}
