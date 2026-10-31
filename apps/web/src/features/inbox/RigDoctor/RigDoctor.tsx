import { Button, Icon, TopBar } from '@maquette/ui';
import type { RigRecord } from '@/ipc/client';
import { isRunning, nextVersion } from '@/state/rig';
import { useRigActions } from './RigDoctor.hooks';
import { BeforeCentre, DefectsAside } from './RigBefore';
import { CompareCentre, FixAside } from './RigCompare';
import { CreatureAside, CreatureCentre } from './RigCreature';
import { RigRefused, RigStopped } from './RigRefused';
import { Facts, Footer, Steps } from './parts';

function count(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);
}

/** The left column: where the case stands and what the model is. */
function LeftColumn({ record }: { record: RigRecord }) {
  const m = record.model;
  const creature = record.creature;
  return (
    <aside
      aria-label="Rig doctor"
      className="flex w-[268px] flex-none flex-col gap-3.5 overflow-y-auto border-r border-line p-3.5"
    >
      <div className="flex items-baseline justify-between px-1">
        <h2 className="m-0 text-panel-title">Rig doctor</h2>
        <span className="text-meta text-fg-mute">{record.route ?? 'no route'}</span>
      </div>
      <Steps record={record} />
      {m.oneIsland ? (
        <div className="rounded-[9px] border border-amber/40 bg-amber-soft p-3 text-meta text-fg-dim">
          <p className="m-0 mb-1 font-medium text-amber">One continuous mesh</p>
          The per island fix would tear it, so it stays off. A narrow reweight moves only the arm
          and hand weight far from the arms.
        </div>
      ) : m.finding ? (
        <div className="rounded-[9px] border border-amber/40 bg-amber-soft p-3 text-meta text-fg-dim">
          <p className="m-0 mb-1 flex items-center gap-1.5 font-medium text-fg">
            <Icon name="warning" size={13} className="text-amber" />
            From the Inbox check
          </p>
          {m.finding}
        </div>
      ) : null}
      {creature ? (
        <Facts
          title="Read from the renders"
          rows={[
            ['Front', creature.front],
            ['Legs', `${creature.legs}${creature.swapped ? `, ${creature.swapped} swapped` : ''}`],
            ['Junk chains', creature.junk.length ? creature.junk.join(', ') : 'none'],
            ['Rigid pieces', creature.oneShell ? 'one shell' : creature.rigidPieces],
          ]}
        />
      ) : (
        <Facts
          title="The model"
          rows={[
            ['Rig', `${m.rig}, ${m.joints} joints`],
            ['Skinned meshes', m.skinnedMeshes],
            ['Clips', m.clips.length ? m.clips.join(', ') : 'none'],
            ['Triangles', count(m.triangles)],
            ['Height', m.height ? `${m.height.toFixed(2)} m` : 'unknown'],
          ]}
        />
      )}
      <p className="m-0 px-1 text-meta text-fg-mute">
        Route picked from the bone names:{' '}
        {record.route === 'creature' ? 'its own chains, legs and junk lists.' : 'Mixamo names.'}
      </p>
    </aside>
  );
}

/** What the person can do next, as the case stands. */
function Actions({ record }: { record: RigRecord }) {
  const s = useRigActions();
  const project = s.project;
  const n = nextVersion(record);
  const was = n - 1;
  if (isRunning(record)) {
    return (
      <>
        <Button variant="secondary" onClick={() => void s.cancel()}>
          Cancel run
        </Button>
        <Button variant="primary" disabled>
          Add as v{n}
        </Button>
      </>
    );
  }
  if (record.source.version === null) {
    return (
      <>
        <Button variant="ghost" onClick={s.leave}>
          Back to the Inbox
        </Button>
        <Button
          variant="primary"
          disabled={!project || s.busy}
          loading={s.busy}
          onClick={() => project && void s.accept(project, null)}
        >
          Accept as v1
        </Button>
      </>
    );
  }
  if (record.state === 'fixed') {
    return (
      <>
        <Button variant="ghost" disabled={s.busy} onClick={() => void s.discard()}>
          Discard draft
        </Button>
        <Button
          variant="secondary"
          icon="arrow-clockwise"
          disabled={s.busy || record.round >= 2 || record.model.oneIsland}
          tooltip={
            record.model.oneIsland
              ? 'A one island mesh has no second round'
              : 'One last try with a wider seam'
          }
          onClick={() => void s.run('round2')}
        >
          Round 2
        </Button>
        <Button variant="primary" disabled={s.busy} loading={s.busy} onClick={() => void s.land()}>
          Add as v{n}
        </Button>
      </>
    );
  }
  if (record.state === 'tested') {
    return (
      <>
        <Button variant="ghost" onClick={s.leave}>
          Keep v{was} as is
        </Button>
        <Button variant="secondary" icon="download-simple" onClick={() => void s.saveSheet()}>
          Save sheet
        </Button>
        <Button variant="primary" disabled={s.busy} onClick={() => void s.run('fix')}>
          Run the fix
        </Button>
      </>
    );
  }
  if (record.state === 'landed' || record.state === 'discarded') {
    return (
      <Button variant="secondary" onClick={s.leave}>
        Back to the Inbox
      </Button>
    );
  }
  return (
    <>
      <Button variant="ghost" onClick={s.leave}>
        Keep v{was} as is
      </Button>
      <Button variant="primary" disabled={s.busy} onClick={() => void s.run('pose-test')}>
        Run the pose test
      </Button>
    </>
  );
}

function footerNote(record: RigRecord): string {
  const v = nextVersion(record) - 1;
  if (record.error) return record.error;
  if (record.landed !== null) return `Added as v${record.landed}. v${v} stays.`;
  if (record.state === 'discarded') return `The draft is gone. v${v} is as it was.`;
  if (record.state === 'cancelled') return `The run was cancelled. v${v} stays as it came.`;
  if (isRunning(record))
    return 'You can leave this view. The run goes on and the Inbox shows its progress.';
  if (record.state === 'fixed') {
    return `Add as v${v + 1} sends the draft through the Inbox checks. v${v} stays, Compare works as for a redo.`;
  }
  if (record.before) {
    const seconds = Math.round(record.before.seconds ?? 0);
    return `Pose test ran headless in ${seconds} s on a copy. v${v} stays as it came.`;
  }
  if (record.source.version === null)
    return 'The pose test compares against v1, so the model is accepted first. The original stays as it came.';
  return 'The pose test takes minutes in headless Blender, on a copy. It spends no credits.';
}

/** The case's working layout: left, centre and right, with the bar under them. */
function Working({ record }: { record: RigRecord }) {
  const compare = record.state === 'fixed' || record.state === 'fixing' || record.after !== null;
  const creature = record.route === 'creature';
  return (
    <>
      <LeftColumn record={record} />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex min-h-0 flex-1">
          {creature ? (
            <>
              <CreatureCentre record={record} />
              {record.state === 'fixed' ? (
                <FixAside record={record} />
              ) : (
                <CreatureAside record={record} />
              )}
            </>
          ) : compare ? (
            <>
              <CompareCentre record={record} />
              <FixAside record={record} />
            </>
          ) : (
            <>
              <BeforeCentre record={record} />
              <DefectsAside record={record} />
            </>
          )}
        </div>
        <Footer note={footerNote(record)}>
          <Actions record={record} />
        </Footer>
      </div>
    </>
  );
}

/** The rig doctor, shown inside the Inbox view for one case. */
export default function RigDoctor({ record }: { record: RigRecord }) {
  const s = useRigActions();
  const version = record.source.version;
  return (
    <>
      <TopBar
        lead={
          <nav aria-label="Where you are" className="flex items-center gap-2 text-fg-mute">
            <button type="button" className="cursor-pointer hover:text-fg" onClick={s.leave}>
              Inbox
            </button>
            <Icon name="caret-right" size={11} />
            <span>{record.name}</span>
            <Icon name="caret-right" size={11} />
            <h1 className="m-0 text-panel-title text-fg">Rig doctor</h1>
          </nav>
        }
        right={
          <div className="flex items-center gap-2">
            {version !== null ? (
              <span className="inline-flex items-center gap-1 rounded-full border border-line px-2 py-0.5 text-meta text-sage">
                <Icon name="check" size={11} />
                Accepted as v{version}
              </span>
            ) : null}
            {record.after ? (
              <Button variant="secondary" icon="download-simple" onClick={() => void s.saveSheet()}>
                Save before and after sheet
              </Button>
            ) : null}
            <Button
              variant="secondary"
              icon="arrow-square-out"
              onClick={() => void s.openBlender()}
            >
              Open in Blender
            </Button>
          </div>
        }
      />
      <main className="flex min-h-0 flex-1">
        {record.state === 'refused' ? (
          <RigRefused record={record} />
        ) : record.state === 'stopped' ? (
          <RigStopped record={record} />
        ) : (
          <Working record={record} />
        )}
      </main>
      <div className="flex flex-none items-center justify-between border-t border-line px-4 py-1 text-meta text-fg-mute">
        <span className="truncate font-mono">{record.dir}</span>
        <span className="flex-none">No credits spent</span>
      </div>
    </>
  );
}
