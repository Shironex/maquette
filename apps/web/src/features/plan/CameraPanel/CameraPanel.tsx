import { Button, cn, Icon, IconButton, InlineBanner } from '@maquette/ui';
import { ApplyReview } from '../ApplyReview';
import { CameraCompare } from '../CameraCompare';
import { countsLine, fieldsLine } from '@/state/plan-camera';
import { useCameraPanel } from './CameraPanel.hooks';
import { StopEditor } from './StopEditor';

const placeNote = (from: number | null, to: number | null, added: boolean) =>
  added ? 'new stop' : from === to ? 'same' : `moves ${from! > to! ? 'up' : 'down'} from ${from}`;

/**
 * The camera plan's side panel: whether `layout.json` changed underneath
 * (compare, rebase or start over), the order Apply would write with what
 * moves, comes and goes, the selected stop's settings, agents' proposals
 * to take or discard, and Add stop and Apply. Nothing reaches
 * `layout.json` until Apply, through its review.
 */
export default function CameraPanel({ onClose, hidden }: { onClose: () => void; hidden: boolean }) {
  const v = useCameraPanel();
  const diff = v.review?.diff;
  const order = (diff?.stops ?? [])
    .filter(s => !s.removed)
    .map(s => (
      <li key={s.id} className="flex items-baseline gap-2">
        <span className="w-5 flex-none text-right font-mono text-[11.5px] text-fg-mute">
          {s.to}
        </span>
        <span className="min-w-0 flex-1 truncate text-fg">{s.name}</span>
        <span
          className={cn(
            'flex-none text-[11.5px]',
            s.added || s.moved ? 'text-clay-text' : s.fields.length ? 'text-amber' : 'text-fg-mute'
          )}
        >
          {placeNote(s.from, s.to, s.added)}
          {fieldsLine(s.fields, false)}
        </span>
      </li>
    ));
  const gone = (diff?.stops ?? []).filter(s => s.removed).map(s => s.name);
  const proposals = v.proposals.map((line, i) => (
    <div
      key={line.nodes[0] ?? i}
      role="group"
      aria-label={`Proposal by ${line.agent}`}
      className="plan-run is-agent"
    >
      <div className="plan-run-head">
        <span className="plan-run-avatar" aria-hidden="true">
          <Icon name="robot" size={15} />
        </span>
        <div className="min-w-0">
          <h3 className="truncate text-[13px] leading-[17px] font-semibold text-fg">Proposal</h3>
          <p className="text-[11.5px] leading-[15px] text-fg-mute">
            {line.agent} · {line.nodes.length} stops
          </p>
        </div>
      </div>
      {line.problem ? <p className="px-3 text-[11.5px] text-amber">{line.problem}</p> : null}
      <div className="plan-run-actions">
        <Button
          size="sm"
          disabled={Boolean(line.problem)}
          disabledReason="Fix the line first"
          tooltip="Sets the main line to this order and these settings; Undo puts yours back"
          onClick={() => v.adopt(line.nodes)}
        >
          Use this order
        </Button>
        <Button size="sm" variant="ghost" onClick={() => v.discard(line.nodes)}>
          Discard
        </Button>
      </div>
    </div>
  ));
  return (
    <aside aria-label="Camera plan" className="plan-side" hidden={hidden}>
      <div className="plan-side-head">
        <h2 className="flex-1 text-[15px] leading-5 font-semibold text-fg">Camera plan</h2>
        <IconButton
          icon="x"
          size="sm"
          variant="ghost"
          label="Close the camera panel"
          onClick={onClose}
        />
      </div>
      <div className="plan-side-body">
        {v.error ? (
          <p role="alert" className="text-meta text-danger">
            The camera plan could not be read. {v.error}
          </p>
        ) : null}
        {v.stale ? (
          <InlineBanner
            tone="warning"
            title="layout.json changed since this plan was based on it"
            detail={`${v.view?.outside.lines.length ? countsLine(v.view.outside) : 'Its stops changed'}. Nothing was written over; compare to see both sides.`}
          />
        ) : null}
        {v.stale ? (
          <div className="flex flex-wrap gap-1.5">
            <Button size="sm" icon="git-branch" onClick={v.compare}>
              Compare
            </Button>
            <Button
              size="sm"
              tooltip="Takes what changed in layout.json and keeps your edits"
              onClick={v.rebase}
            >
              Rebase
            </Button>
            <Button
              size="sm"
              variant="ghost"
              tooltip="Builds the main line again from layout.json; Undo brings yours back"
              onClick={v.startOver}
            >
              Start over
            </Button>
          </div>
        ) : null}
        {v.view?.problem ? <InlineBanner tone="warning" title={v.view.problem} /> : null}
        <section aria-label="Order after Apply" className="flex flex-col gap-1.5">
          <h3 className="text-[13px] font-semibold text-fg">
            Order after Apply
            <span className="ml-1.5 font-normal text-fg-mute">{order.length} stops</span>
          </h3>
          {order.length ? <ol className="flex flex-col gap-1 text-[12.5px]">{order}</ol> : null}
          {gone.length ? (
            <p className="text-[12px] text-danger">Removed: {gone.join(', ')}</p>
          ) : null}
          {diff ? <p className="text-[11.5px] text-fg-mute">{countsLine(diff)}</p> : null}
        </section>
        {v.selected ? <StopEditor key={v.selected} nodeId={v.selected} /> : null}
        {proposals.length ? proposals : null}
      </div>
      <div className="plan-side-foot flex flex-col gap-2">
        <p>
          Nothing changes on the site until you press Apply. The old layout.json is kept in
          plans/history, so Undo in Changes puts it back.
        </p>
        <div className="flex flex-wrap gap-1.5">
          <Button
            size="sm"
            icon="plus"
            disabled={!v.canAdd}
            disabledReason="A tour holds at most 24 stops"
            onClick={v.addStop}
          >
            Add stop
          </Button>
          <span className="flex-1" />
          <Button
            size="sm"
            variant="primary"
            icon="upload-simple"
            disabled={v.applyOff !== null}
            disabledReason={v.applyOff ?? undefined}
            onClick={v.apply}
          >
            Apply to layout.json
          </Button>
        </div>
      </div>
      <ApplyReview />
      <CameraCompare />
    </aside>
  );
}
