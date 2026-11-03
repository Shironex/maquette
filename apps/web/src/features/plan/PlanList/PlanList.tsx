import { Button, EmptyState, Icon, InlineBanner, TextField, TopBar } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import type { PlanSummary } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { PRESET_NAME } from '@/state/plan-presets';
import { contentLine } from '@/state/plans';
import { AgentPrompt } from '../AgentPrompt';
import { NewCanvasDialog } from '../NewCanvasDialog';
import { PresetPicker, PresetTag } from '../PresetPicker';
import { RemoveCanvasDialog } from '../RemoveCanvasDialog';
import { usePlanList } from './PlanList.hooks';
import { PlanRow } from './PlanRow';

export interface PlanListProps {
  list: readonly PlanSummary[];
  /** The canvas being opened, if any. */
  opening: string | null;
  openError: string | null;
}

/** What the empty list offers to paste into an agent; the brackets are for the person to fill. */
const START_PROMPT =
  'Make a canvas in this Maquette project that plans <what you want to plan>, using the Plan tools.';

const edited = (plan: PlanSummary, now: number) =>
  plan.edited ? formatAgo(Date.parse(plan.edited), now) : 'unknown';

/**
 * A project's canvases: the ones edited last as cards, then all of them in
 * a table with rename and remove. Each canvas is one file in `plans/`.
 */
export default function PlanList({ list, opening, openError }: PlanListProps) {
  const v = usePlanList(list);
  const count = `${list.length} ${list.length === 1 ? 'canvas' : 'canvases'}`;
  const cards = v.recent.map(plan => (
    <button
      key={plan.id}
      type="button"
      aria-busy={opening === plan.id}
      onClick={() => v.open(plan.id)}
      className="flex min-w-0 flex-col overflow-hidden rounded-card border border-line bg-raised text-left hover:border-line-strong focus-visible:outline-2 focus-visible:outline-focus"
    >
      <span className="plan-dots relative h-[104px] border-b border-line" aria-hidden="true">
        <span className="absolute right-2.5 bottom-2 font-mono text-[11px] text-fg-mute">
          {plan.nodes} {plan.nodes === 1 ? 'node' : 'nodes'}
        </span>
      </span>
      <span className="flex flex-col gap-1 px-3 pt-2.5 pb-3">
        {/* The spaces keep the card's name readable ("Combat ideas Story 6 nodes ..."). */}
        {/* A long name takes two lines beside its preset, then ends in an ellipsis; hover shows it all. */}
        <span className="flex min-w-0 items-start gap-2">
          <span
            title={plan.name}
            className="line-clamp-2 min-w-0 flex-1 text-[13.5px] leading-5 font-semibold break-words text-fg"
          >
            {plan.name}
          </span>{' '}
          {plan.preset ? <PresetTag>{PRESET_NAME[plan.preset]}</PresetTag> : null}
        </span>{' '}
        <span className="truncate font-mono text-[11.5px] text-fg-mute">{contentLine(plan)}</span>{' '}
        <span className="text-meta text-fg-dim">Edited {edited(plan, v.now)}</span>
      </span>
    </button>
  ));
  const pinned = v.camera ? (
    <section aria-label="Pinned" className="flex flex-col gap-3">
      <SectionLabel>Pinned</SectionLabel>
      <button
        type="button"
        aria-busy={opening === 'camera-plan'}
        onClick={v.openCamera}
        className="flex max-w-[460px] items-center gap-3 rounded-card border border-line bg-raised px-3.5 py-3 text-left hover:border-line-strong focus-visible:outline-2 focus-visible:outline-focus"
      >
        <span className="grid size-10 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
          <Icon name="video-camera" size={18} />
        </span>
        <span className="flex min-w-0 flex-col gap-0.5">
          <span className="truncate text-[13.5px] font-semibold text-fg">Camera plan</span>{' '}
          <span className="truncate text-meta text-fg-dim">
            {v.camera === 'new'
              ? 'Plan the tour from the stops in layout.json'
              : `${contentLine(v.camera)} · edited ${edited(v.camera, v.now)}`}
          </span>
        </span>
      </button>
    </section>
  ) : null;
  const rows = v.found.map(plan => (
    <PlanRow
      key={plan.id}
      plan={plan}
      edited={edited(plan, v.now)}
      renaming={v.renaming?.id === plan.id ? v.renaming.name : null}
      onOpen={() => v.open(plan.id)}
      onRename={() => v.startRename(plan)}
      onType={v.typeName}
      onCommit={() => void v.commitRename()}
      onCancel={v.cancelRename}
      onRemove={() => v.askRemove(plan)}
    />
  ));
  const newButton = (
    <Button
      variant="primary"
      icon="plus"
      tooltip="Makes a canvas from a preset"
      onClick={() => v.making.open()}
    >
      New canvas
    </Button>
  );
  const firstCanvas = (
    <div className="flex w-full flex-col gap-3">
      <PresetPicker value={v.making.start} onChange={v.making.setStart} />
      <Button variant="primary" icon="plus" onClick={() => v.making.open(v.making.start)}>
        {`Create ${PRESET_NAME[v.making.start]} canvas`}
      </Button>
      <AgentPrompt
        title="Or let an agent start one"
        prompt={START_PROMPT}
        hint="Agents use the Plan tools over MCP."
      />
    </div>
  );
  return (
    <>
      <TopBar
        lead={
          <div className="flex items-baseline gap-3">
            <h1 className="text-panel-title">Plan</h1>
            <span className="text-meta text-fg-mute">{count}</span>
          </div>
        }
        right={
          <>
            {list.length > 0 ? (
              <TextField
                aria-label="Find a canvas"
                placeholder="Find a canvas"
                icon="magnifying-glass"
                value={v.query}
                onChange={e => v.setQuery(e.target.value)}
                fieldClassName="w-[220px]"
              />
            ) : null}
            {newButton}
          </>
        }
      />
      <main className="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 py-5">
        {openError ? (
          <InlineBanner tone="danger" title="The canvas could not be opened" detail={openError} />
        ) : null}
        {pinned}
        {list.length === 0 ? (
          <div className="grid flex-1 place-items-center">
            <EmptyState
              icon="graph"
              title="Plan with canvases"
              body="Notes, pictures, links and your assets in one place. Agents can add to it, and you see every change they make."
              action={firstCanvas}
            />
          </div>
        ) : null}
        {v.recent.length > 0 ? (
          <section aria-label="Recent canvases" className="flex flex-col gap-3">
            <div className="flex items-baseline justify-between gap-4">
              <SectionLabel>Recent</SectionLabel>
              <span className="text-meta text-fg-mute">
                Each canvas is one file in plans/ in the project folder
              </span>
            </div>
            <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3.5">
              {cards}
            </div>
          </section>
        ) : null}
        {list.length > 0 ? (
          <section aria-label="All canvases" className="flex flex-col gap-2">
            <SectionLabel>All canvases</SectionLabel>
            <table className="w-full table-fixed border-collapse text-[12.5px]">
              <colgroup>
                <col />
                <col className="w-[132px]" />
                <col className="w-[72px]" />
                <col className="w-[132px]" />
                <col className="w-[34%]" />
                <col className="w-[84px]" />
              </colgroup>
              <thead>
                <tr className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
                  <th className="py-1.5 pl-3 text-left font-semibold">Name</th>
                  <th className="px-2 py-1.5 text-left font-semibold">Preset</th>
                  <th className="px-2 py-1.5 text-right font-semibold">Nodes</th>
                  <th className="px-2 py-1.5 text-left font-semibold">Edited</th>
                  <th className="px-2 py-1.5 text-left font-semibold">File</th>
                  <th className="py-1.5 pr-3 text-right font-semibold">
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {v.found.length === 0 ? (
                  <tr className="border-t border-line">
                    <td colSpan={6} className="py-2.5 pl-3 text-fg-mute">
                      No canvas is called that.
                    </td>
                  </tr>
                ) : null}
                {rows}
              </tbody>
            </table>
          </section>
        ) : null}
      </main>
      {v.making.preset ? (
        <NewCanvasDialog preset={v.making.preset} onClose={v.making.close} />
      ) : null}
      {v.removing ? <RemoveCanvasDialog plan={v.removing} onClose={v.closeRemove} /> : null}
    </>
  );
}
