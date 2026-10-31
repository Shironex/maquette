import { Button, Icon, SegmentedControl, cn, type SegmentedOption } from '@maquette/ui';
import type { RigRecord, RigRow } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import {
  cellAspect,
  changeLines,
  changedRows,
  nextVersion,
  rowFlagged,
  rowNumbers,
} from '@/state/rig';
import { Enlarges } from './RigEnlarge';
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
  span,
}: {
  before: string | null;
  after: string | null;
  label: string;
  span: number;
}) {
  const [at, setAt] = useChoice(50);
  return (
    <div
      style={{ gridColumn: `span ${span}` }}
      className="relative aspect-[3/4] max-h-[520px] overflow-hidden rounded-[8px] border border-line bg-studio"
    >
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

const TREND_TONE = { better: 'text-sage', worse: 'text-amber', same: 'text-fg-mute' };

/** What a row's numbers did, in words and the colour of their direction. */
export function RowChange({ row }: { row: RigRow }) {
  const lines = changeLines(row);
  if (lines.length === 0) {
    return <p className="m-0 text-meta text-fg-mute">{row.note ?? 'no numbers for this row'}</p>;
  }
  const said = lines.map(l => (
    <p key={l.label} className={cn('m-0 text-meta', TREND_TONE[l.trend])}>
      {l.label} {l.text}
    </p>
  ));
  return (
    <>
      {said}
      {row.beyondBodyMm == null ? null : (
        <p className="m-0 text-meta text-fg-mute">
          past body {Math.round(row.change?.beyondBodyBefore ?? 0)} to{' '}
          {Math.round(row.beyondBodyMm)} mm
        </p>
      )}
    </>
  );
}

function Row({ record, row, mode }: { record: RigRecord; row: RigRow; mode: Mode }) {
  const was = record.before?.rows.find(r => r.id === row.id);
  const move = record.jointMove;
  const moved = move?.sheet?.rows.find(r => r.id === row.id);
  const front = fileUrl(row.beforeFront ?? was?.front);
  const pages = [
    {
      label: 'Front',
      pictures: [
        { url: front, caption: 'Before' },
        { url: fileUrl(row.front), caption: 'Draft' },
      ],
    },
    {
      label: 'Side',
      pictures: [
        { url: fileUrl(row.beforeSide ?? was?.side), caption: 'Before' },
        { url: fileUrl(row.side), caption: 'Draft' },
      ],
    },
  ];
  return (
    <>
      <div className="min-w-0 self-center font-mono">
        <p className="m-0 text-fg-dim">{row.label}</p>
        <RowChange row={row} />
      </div>
      {mode === 'slider' ? (
        <Slider
          before={front}
          after={fileUrl(row.front)}
          label={row.label}
          span={move?.sheet ? 3 : 2}
        />
      ) : (
        <>
          <Enlarges title={row.label} detail={row.note ?? rowNumbers(row)} pages={pages}>
            <Tile
              url={front}
              alt={`${row.label}, before`}
              flagged={rowFlaggedIn(was)}
              aspect={cellAspect(record)}
            />
          </Enlarges>
          <Enlarges title={row.label} detail={row.note ?? rowNumbers(row)} pages={pages}>
            <Tile
              url={fileUrl(row.front)}
              alt={`${row.label}, draft`}
              aspect={cellAspect(record)}
            />
          </Enlarges>
          {move?.sheet ? (
            <Tile
              url={fileUrl(moved?.front)}
              alt={`${row.label}, joints moved`}
              dim={move.state !== 'approved'}
              aspect={cellAspect(record)}
            />
          ) : null}
        </>
      )}
    </>
  );
}

const rowFlaggedIn = (row: RigRow | undefined) => (row ? rowFlagged(row) : false);

/** The centre of the second board: the same poses, before and draft. */
export function CompareCentre({ record }: { record: RigRecord }) {
  const [mode, setMode] = useChoice<Mode>('side');
  const rows = record.after?.rows ?? [];
  const changed = changedRows(record);
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
      <div
        className={cn(
          'grid flex-1 content-start items-start gap-x-2.5 gap-y-3',
          move?.sheet ? 'grid-cols-[170px_1fr_1fr_1fr]' : 'grid-cols-[170px_1fr_1fr]'
        )}
      >
        <span />
        <span className="text-caption tracking-[0.06em] text-fg-mute uppercase">
          v{was} · before
        </span>
        <span className="text-caption tracking-[0.06em] text-fg-mute uppercase">
          Draft v{n} · weights only
        </span>
        {move?.sheet ? (
          <span className="inline-flex items-center gap-1 text-caption tracking-[0.06em] text-fg-mute uppercase">
            With joints moved
            {move.state === 'approved' ? null : <Icon name="clock" size={11} />}
          </span>
        ) : null}
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
