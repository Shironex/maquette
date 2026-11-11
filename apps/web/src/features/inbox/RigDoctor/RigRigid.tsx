import { Button, Checkbox, Icon, Select } from '@maquette/ui';
import type { RigPart, RigRecord, RigRigid } from '@/ipc/client';
import { changeLines, WAIT_FOR_ANSWER } from '@/state/rig';
import { useChoice, useRigActions } from './RigDoctor.hooks';

const bare = (bone: string) => bone.slice(bone.lastIndexOf(':') + 1);
const mm = (n: number | null) => Math.round(n ?? 0);
const partNames = (parts: number[]) =>
  `${parts.length === 1 ? 'Part' : 'Parts'} ${parts.join(' and ')}`;

/** What the bind did to the pose the fix's draft stretches most in. */
function worstLine(bind: RigRigid): string | null {
  if (!bind.row || bind.stretchBefore == null) return null;
  return `${bind.row}: worst stretch ${mm(bind.stretchBefore)} to ${mm(bind.stretchAfter)} mm`;
}

/** The parts to bind and the bone they follow, picked by a person. */
function Picker({ record, parts }: { record: RigRecord; parts: RigPart[] }) {
  const { bindRigid, busy } = useRigActions();
  const bones = record.bodyBones ?? [];
  const [picked, setPicked] = useChoice<number[]>(parts.map(p => p.part));
  const [bone, setBone] = useChoice(bones.find(b => bare(b) === 'Spine2') ?? bones[0] ?? '');
  const toggle = (n: number, on: boolean) =>
    setPicked(on ? [...picked, n].sort((a, b) => a - b) : picked.filter(p => p !== n));
  const rows = parts.map(p => (
    <li key={p.part} className="flex flex-col gap-0.5 rounded-[9px] border border-line p-2">
      <Checkbox
        checked={picked.includes(p.part)}
        onChange={on => toggle(p.part, on)}
        label={`Part ${p.part}, ${p.side}`}
      />
      <p className="m-0 ml-6 font-mono text-meta text-fg-mute">
        {p.vertices.toLocaleString('en')} vertices, {mm(p.acrossCm)} cm across, on{' '}
        {p.bones.join(', ') || 'the arm'}
      </p>
    </li>
  ));
  return (
    <>
      <ul className="m-0 flex list-none flex-col gap-1.5 p-0">{rows}</ul>
      <div className="flex items-end justify-between gap-2">
        <Select
          size="sm"
          label="Follows"
          options={bones.map(b => ({ value: b, label: bare(b) }))}
          value={bone}
          onChange={setBone}
        />
        <Button
          size="sm"
          variant="secondary"
          icon="bone"
          disabled={busy || picked.length === 0}
          disabledReason={picked.length === 0 ? 'Pick at least one part' : WAIT_FOR_ANSWER}
          onClick={() => void bindRigid(picked, bone || null)}
        >
          Bind rigidly
        </Button>
      </div>
    </>
  );
}

/** A measured bind waiting for a person: the numbers, then use it or keep the fix's weights. */
function Asked({ bind }: { bind: RigRigid }) {
  const { decideRigid, busy } = useRigActions();
  const failed = bind.checks.filter(c => !c.ok);
  const worst = worstLine(bind);
  const moved = (bind.sheet?.rows ?? []).filter(r => r.change && r.change.stretch !== 'same');
  const problems = failed.map(c => (
    <p key={c.title} className="m-0 ml-[42px] text-meta text-danger">
      {c.title}: {c.detail}
    </p>
  ));
  const rows = moved.map(r => (
    <li key={r.id} className="flex justify-between gap-2 font-mono text-meta">
      <span className="text-fg-dim">{r.label}</span>
      <span className={r.change?.stretch === 'worse' ? 'text-amber' : 'text-sage'}>
        {changeLines(r)[0]?.text}
      </span>
    </li>
  ));
  return (
    <section
      role="alertdialog"
      aria-labelledby="rig-rigid-ask"
      className="flex flex-col gap-2 rounded-card border border-amber/40 bg-raised p-3.5"
    >
      <div className="flex gap-2.5">
        <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
          <Icon name="bone" size={17} />
        </span>
        <div className="min-w-0">
          <h3 id="rig-rigid-ask" className="m-0 text-emphasis font-semibold">
            Use the bound parts?
          </h3>
          <p className="m-0 mt-1 text-meta text-fg-mute">
            {bind.askedBy === 'agent' ? 'An agent asked for this. ' : ''}
            {partNames(bind.parts)} ({bind.vertices.toLocaleString('en')} vertices) follow{' '}
            {bare(bind.bone)} alone on a copy of the draft. Only weights changed.
          </p>
        </div>
      </div>
      {worst ? <p className="m-0 ml-[42px] text-meta text-sage">{worst}</p> : null}
      {rows.length > 0 ? (
        <ul
          aria-label="Rows the bind moved"
          className="m-0 ml-[42px] flex list-none flex-col gap-1 p-0"
        >
          {rows}
        </ul>
      ) : null}
      {problems}
      <div className="flex justify-end gap-2">
        <Button
          size="sm"
          variant="secondary"
          disabled={busy}
          disabledReason={WAIT_FOR_ANSWER}
          onClick={() => void decideRigid(false)}
        >
          Keep the fix&apos;s weights
        </Button>
        <Button
          size="sm"
          variant="primary"
          icon="check"
          disabled={busy || failed.length > 0}
          disabledReason={
            failed.length > 0 ? 'A check of the bound draft did not hold' : WAIT_FOR_ANSWER
          }
          onClick={() => void decideRigid(true)}
        >
          Use bound parts
        </Button>
      </div>
    </section>
  );
}

/** A bind in the draft, with the undo that puts the fix's draft back. */
function Bound({ bind }: { bind: RigRigid }) {
  const { decideRigid, busy } = useRigActions();
  const worst = worstLine(bind);
  return (
    <div className="flex flex-col gap-1.5">
      <p className="m-0 flex items-center gap-1.5 text-fg">
        <Icon name="check-circle" size={15} className="flex-none text-sage" />
        {partNames(bind.parts)} follow {bare(bind.bone)} in the draft
      </p>
      {worst ? <p className="m-0 font-mono text-meta text-fg-mute">{worst}</p> : null}
      <div className="flex justify-end">
        <Button
          size="sm"
          variant="secondary"
          icon="arrow-u-up-left"
          disabled={busy}
          disabledReason={WAIT_FOR_ANSWER}
          onClick={() => void decideRigid(false)}
        >
          Undo the bind
        </Button>
      </div>
    </div>
  );
}

/**
 * Rigid parts: what the arm bones hold away from the arms, such as wings or
 * a cape, bound to one body bone on a copy of a humanoid's fixed draft.
 */
export function RigRigid({ record }: { record: RigRecord }) {
  const parts = record.parts ?? [];
  const bind = record.rigid ?? null;
  const live = record.state === 'fixed' || (record.state === 'fixing' && bind?.state === 'making');
  if (record.route !== 'humanoid' || !live || (parts.length === 0 && !bind)) return null;
  if (bind?.state === 'asked') return <Asked bind={bind} />;
  let body;
  if (bind?.state === 'making') {
    body = (
      <p className="m-0 text-meta text-fg-mute">
        Binding {partNames(bind.parts).toLowerCase()} to {bare(bind.bone)} and testing the poses...
      </p>
    );
  } else if (bind?.state === 'approved') {
    body = <Bound bind={bind} />;
  } else {
    body = <Picker record={record} parts={parts} />;
  }
  return (
    <section aria-label="Rigid parts" className="flex flex-col gap-2 border-t border-line pt-2.5">
      <h3 className="m-0 font-medium text-fg">Parts on the arm bones</h3>
      <p className="m-0 text-meta text-fg-mute">
        Held by the arm bones away from the arm, such as wings or a cape. Bound rigidly, they follow
        one body bone instead. Only weights change, on a copy.
      </p>
      {bind?.state === 'declined' ? (
        <p className="m-0 text-meta text-fg-mute">The last bind stayed out of the draft.</p>
      ) : null}
      {body}
    </section>
  );
}
