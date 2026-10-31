import { Button, Icon, SegmentedControl, cn, type SegmentedOption } from '@maquette/ui';
import type { RigRecord, RigRow } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { nextVersion } from '@/state/rig';
import { useChoice, useRigActions } from './RigDoctor.hooks';
import { WalkProof } from './RigCreature';
import { RunProgress, Stats, Tile } from './parts';

type Mode = 'side' | 'changed' | 'slider';

const MODES: readonly SegmentedOption<Mode>[] = [
  { value: 'side', label: 'Side by side' },
  { value: 'changed', label: 'Only changed rows' },
  { value: 'slider', label: 'Slider' },
];

/** The before and the draft over each other, split where the slider says. */
function Slider({
  before,
  after,
  label,
}: {
  before: string | null;
  after: string | null;
  label: string;
}) {
  const [at, setAt] = useChoice(50);
  return (
    <div className="relative col-span-3 h-[220px] overflow-hidden rounded-[8px] border border-line bg-studio">
      {after ? (
        <img
          src={after}
          alt={`${label}, draft`}
          className="absolute inset-0 h-full w-full object-contain"
        />
      ) : null}
      {before ? (
        <img
          src={before}
          alt={`${label}, before`}
          className="absolute inset-0 h-full w-full object-contain"
          style={{ clipPath: `inset(0 ${100 - at}% 0 0)` }}
        />
      ) : null}
      <input
        type="range"
        aria-label={`Before and draft of ${label}`}
        min={0}
        max={100}
        value={at}
        onChange={e => setAt(Number(e.target.value))}
        className="absolute inset-x-3 bottom-2 w-[calc(100%-24px)]"
      />
    </div>
  );
}

function Row({ record, row, mode }: { record: RigRecord; row: RigRow; mode: Mode }) {
  const was = record.before?.rows.find(r => r.id === row.id);
  const moved = record.jointMove?.sheet?.rows.find(r => r.id === row.id);
  const used = record.jointMove?.state === 'approved';
  const flagged = (was?.defects.length ?? 0) > 0;
  const tears = (row.stretchMm ?? 0) >= 40;
  return (
    <>
      <div className="min-w-0 self-center">
        <p className="m-0 font-mono text-fg-dim">{row.label}</p>
        <p className={cn('m-0 text-meta', tears ? 'text-clay-text' : 'text-fg-mute')}>
          {row.note ?? 'unchanged'}
        </p>
      </div>
      {mode === 'slider' ? (
        <Slider before={fileUrl(was?.front)} after={fileUrl(row.front)} label={row.label} />
      ) : (
        <>
          <div className="h-[92px]">
            <Tile url={fileUrl(was?.front)} alt={`${row.label}, before`} flagged={flagged} />
          </div>
          <div className="h-[92px]">
            <Tile url={fileUrl(row.front)} alt={`${row.label}, draft`} />
          </div>
          <div className="h-[92px]">
            <Tile url={fileUrl(moved?.front)} alt={`${row.label}, joints moved`} dim={!used} />
          </div>
        </>
      )}
    </>
  );
}

/** The centre of the second board: the same poses, before and draft. */
export function CompareCentre({ record }: { record: RigRecord }) {
  const [mode, setMode] = useChoice<Mode>('side');
  const rows = record.after?.rows ?? [];
  const changed = rows.filter(r => record.before?.rows.find(b => b.id === r.id)?.defects.length);
  const shown = mode === 'side' ? rows : changed;
  const n = nextVersion(record);
  const move = record.jointMove;
  const was = n - 1;
  const lines = shown.map(row => <Row key={row.id} record={record} row={row} mode={mode} />);
  return (
    <section
      aria-label="Before and draft"
      className="flex min-w-0 flex-1 flex-col gap-3 overflow-y-auto px-5 pt-4 pb-4"
    >
      <div className="flex flex-none items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 className="m-0 text-panel-title">
            {record.name}{' '}
            <span className="ml-1 rounded-[5px] bg-hover px-1.5 py-0.5 font-mono text-meta">
              draft v{n}
            </span>
          </h2>
          <p className="m-0 mt-0.5 text-fg-mute">
            Same poses, same camera · {changed.length} of {rows.length} rows changed
          </p>
        </div>
        <SegmentedControl
          label="Compare"
          tone="neutral"
          options={MODES}
          value={mode}
          onChange={setMode}
        />
      </div>
      <RunProgress record={record} />
      <div className="grid flex-1 grid-cols-[72px_1fr_1fr_1fr] content-start items-stretch gap-x-2.5 gap-y-2">
        <span />
        <span className="text-caption tracking-[0.06em] text-fg-mute uppercase">
          v{was} · before
        </span>
        <span className="text-caption tracking-[0.06em] text-fg-mute uppercase">
          Draft v{n} · weights only
        </span>
        <span className="inline-flex items-center gap-1 text-caption tracking-[0.06em] text-fg-mute uppercase">
          With joints moved
          {move?.state === 'approved' ? null : <Icon name="clock" size={11} />}
        </span>
        {lines}
      </div>
      {record.stats.length > 0 ? <Stats stats={record.stats} /> : null}
    </section>
  );
}

/** The joint move: offered only here, with the clips it would break named. */
export function JointMoveCard({ record }: { record: RigRecord }) {
  const { decideJoints: decide, busy } = useRigActions();
  const move = record.jointMove;
  if (!move || move.state !== 'asked') return null;
  const clips = move.breaksClips;
  const off = Math.round(move.offCm ?? 0);
  return (
    <section
      role="alertdialog"
      aria-labelledby="rig-joint-move"
      className="flex flex-col gap-2 rounded-card border border-amber/40 bg-raised p-3.5"
    >
      <div className="flex gap-2.5">
        <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
          <Icon name="person-arms-spread" size={17} />
        </span>
        <div className="min-w-0">
          <h3 id="rig-joint-move" className="m-0 text-emphasis font-semibold">
            Also move the {move.limb} joints?
          </h3>
          <p className="m-0 mt-1 text-meta text-fg-mute">
            The chain sits {off} cm off its mesh. Moving {move.joints} joints onto it changes the
            rest pose. Weights only already frees the cloth.
          </p>
        </div>
      </div>
      <p className="m-0 ml-[42px] text-meta text-amber">
        {clips.length > 0
          ? `This would break ${clips.join(' and ')}: ${clips.length === 1 ? 'that clip has' : 'those clips have'} translation tracks on the ${move.limb} and would pull it back.`
          : `No clip of this model has translation tracks on the ${move.limb}, so none would break.`}
      </p>
      <div className="flex justify-end gap-2">
        <Button size="sm" variant="secondary" disabled={busy} onClick={() => void decide(false)}>
          Keep weights only
        </Button>
        <Button
          size="sm"
          variant="primary"
          icon="check"
          disabled={busy}
          onClick={() => void decide(true)}
        >
          Approve and move
        </Button>
      </div>
    </section>
  );
}

/** The right column of the second board: what the fix did and proved. */
export function FixAside({ record }: { record: RigRecord }) {
  const seconds = Math.round(record.stages.reduce((sum, s) => sum + (s.seconds ?? 0), 0));
  const steps = record.fixSteps.map((s, i) => (
    <li key={s.title} className="flex gap-2.5 rounded-[9px] border border-line bg-raised p-2.5">
      <span className="grid size-5 flex-none place-items-center rounded-full border border-line-strong text-caption text-fg-mute">
        {i + 1}
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-2">
          <h4 className="m-0 font-medium text-fg">{s.title}</h4>
          <span className="flex-none font-mono text-meta text-fg-dim">{s.value}</span>
        </div>
        <p className="m-0 text-meta text-fg-mute">{s.detail}</p>
      </div>
    </li>
  ));
  const checks = record.checks.map(c => (
    <li key={c.title} className="flex gap-2.5 border-t border-line py-2 first:border-t-0">
      <Icon
        name={c.ok ? 'check-circle' : 'warning-circle'}
        size={16}
        className={c.ok ? 'mt-0.5 flex-none text-sage' : 'mt-0.5 flex-none text-danger'}
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-2">
          <p className="m-0 font-medium text-fg">{c.title}</p>
          <span className="flex-none font-mono text-meta text-fg-dim">{c.value}</span>
        </div>
        <p className="m-0 text-meta text-fg-mute">{c.detail}</p>
      </div>
    </li>
  ));
  return (
    <aside
      aria-label="Fix and verify"
      className="flex w-[350px] flex-none flex-col gap-2.5 overflow-y-auto border-l border-line p-4 [&>*]:flex-none"
    >
      <div className="flex items-center justify-between">
        <h2 className="m-0 text-panel-title">Fix and verify</h2>
        {seconds > 0 ? (
          <span className="inline-flex items-center gap-1 rounded-full bg-sage-soft px-2 py-0.5 text-meta text-sage">
            <Icon name="check" size={11} />
            ran in {seconds} s
          </span>
        ) : null}
      </div>
      <ol className="m-0 flex list-none flex-col gap-1.5 p-0">{steps}</ol>
      <ul className="m-0 flex list-none flex-col p-0" aria-label="Verify checks">
        {checks}
      </ul>
      <JointMoveCard record={record} />
      <WalkProof record={record} />
    </aside>
  );
}
