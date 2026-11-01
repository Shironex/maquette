import { Button, Icon } from '@maquette/ui';
import type { RigRecord } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { acceptWaitsFor } from '@/state/rig';
import { useRigActions, useRigRedo } from './RigDoctor.hooks';
import { Footer, Tile } from './parts';

/** The stops that are not this model's, so a person knows what else ends a case. */
const OTHER_STOPS = [
  ['No skin', 'A model without weights has nothing to fix. Redo it as rigged.'],
  ['A check got worse after the fix', 'The draft is held and the version stays.'],
  [
    'The fix needs a mesh, UV or texture edit',
    'The doctor changes weights only. Finish it in Blender.',
  ],
];

/** The fourth board's first card: a model the doctor cannot fix, and why. */
export function RigRefused({ record }: { record: RigRecord }) {
  const { openBlender, leave, accept, busy, project } = useRigActions();
  const redo = useRigRedo(record);
  const issues = record.issues.map(i => (
    <li key={i.title} className="flex gap-2.5 border-t border-line py-2 first:border-t-0">
      <Icon
        name={i.ok ? 'check-circle' : 'warning-circle'}
        size={16}
        className={i.ok ? 'mt-0.5 flex-none text-sage' : 'mt-0.5 flex-none text-clay-text'}
      />
      <div className="min-w-0">
        <p className="m-0 font-medium text-fg">{i.title}</p>
        <p className="m-0 text-meta text-fg-mute">{i.detail}</p>
      </div>
    </li>
  ));
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1 justify-center overflow-y-auto p-6">
        <section className="flex w-full max-w-[720px] flex-col gap-3 self-start rounded-card border border-line bg-panel p-5">
          <h2 className="m-0 text-panel-title">The rig doctor cannot fix {record.name}</h2>
          <ul className="m-0 flex list-none flex-col p-0" aria-label="What the doctor read">
            {issues}
          </ul>
          <h3 className="m-0 text-caption tracking-[0.06em] text-fg-mute uppercase">
            What you can do
          </h3>
          <div className="grid grid-cols-2 gap-2.5">
            <div className="rounded-[9px] border border-line bg-raised p-3">
              <p className="m-0 font-medium text-fg">Redo it as rigged</p>
              <p className="m-0 mt-1 mb-2 text-meta text-fg-mute">
                Send the asset again with rig on and one mesh. That spends credits, so it is your
                call in the send sheet. The doctor never starts one.
              </p>
              <Button
                size="sm"
                variant="secondary"
                icon="arrow-bend-up-right"
                disabled={Boolean(redo.blocker)}
                disabledReason={redo.blocker ?? undefined}
                onClick={redo.open}
              >
                Redo...
              </Button>
            </div>
            <div className="rounded-[9px] border border-line bg-raised p-3">
              <p className="m-0 font-medium text-fg">Join the meshes in Blender</p>
              <p className="m-0 mt-1 mb-2 text-meta text-fg-mute">
                Then add the fixed file to the same asset.
              </p>
              <Button
                size="sm"
                variant="secondary"
                icon="arrow-square-out"
                onClick={() => void openBlender()}
              >
                Open in Blender
              </Button>
            </div>
          </div>
        </section>
      </div>
      <Footer note="A model the doctor refuses keeps its Inbox checks. Accept as is is always one click away.">
        <Button variant="ghost" onClick={leave}>
          Back to the Inbox
        </Button>
        {record.source.inbox ? (
          <Button
            variant="secondary"
            disabled={!project || busy}
            disabledReason={acceptWaitsFor(project)}
            loading={busy}
            tooltip="Accepts the model as it came; its checks stay on the row"
            onClick={() => project && void accept(project, null)}
          >
            Accept as is
          </Button>
        ) : null}
      </Footer>
    </div>
  );
}

/** The fourth board's second card: the doctor stopped and says where. */
export function RigStopped({ record }: { record: RigRecord }) {
  const { discard, saveSheet, openBlender, leave, busy } = useRigActions();
  const stop = record.stop;
  const row = record.after?.rows.find(r => r.id === stop?.row);
  const was = record.before?.rows.find(r => r.id === stop?.row);
  const others = OTHER_STOPS.filter(([t]) => t !== stop?.title).map(([title, detail]) => (
    <li key={title}>
      <p className="m-0 font-medium text-fg-dim">{title}</p>
      <p className="m-0 text-fg-mute">{detail}</p>
    </li>
  ));
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1 justify-center gap-4 overflow-y-auto p-6">
        <section className="flex w-full max-w-[620px] flex-col gap-3 self-start rounded-card border border-clay-line bg-panel p-5">
          <h2 className="m-0 text-panel-title">{stop?.title ?? `Stopped: ${record.name}`}</h2>
          <p className="m-0 text-fg-dim">{stop?.detail}</p>
          {row ? (
            <div className="grid h-[220px] grid-cols-2 gap-2">
              <Tile
                url={fileUrl(was?.front)}
                alt={`${row.label}, v${record.source.version ?? 1}`}
              />
              <Tile url={fileUrl(row.front)} alt={`${row.label}, round ${record.round}`} flagged />
            </div>
          ) : null}
          <p className="m-0 text-meta text-fg-mute">The draft is kept, not added.</p>
          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              disabled={busy}
              disabledReason="Wait for the answer on its way"
              onClick={() => void discard()}
            >
              Discard draft
            </Button>
            <Button variant="secondary" icon="download-simple" onClick={() => void saveSheet()}>
              Save sheets
            </Button>
            <Button variant="primary" icon="arrow-square-out" onClick={() => void openBlender()}>
              Open draft in Blender
            </Button>
          </div>
        </section>
        <aside className="w-[280px] flex-none self-start rounded-card border border-line p-4">
          <h3 className="m-0 mb-2 text-caption tracking-[0.06em] text-fg-mute uppercase">
            Other stops
          </h3>
          <ul className="m-0 flex list-none flex-col gap-2 p-0 text-meta">{others}</ul>
        </aside>
      </div>
      <Footer note="The doctor never guesses a third time. v1 stays as it came.">
        <Button variant="secondary" onClick={leave}>
          Back to the Inbox
        </Button>
      </Footer>
    </div>
  );
}
