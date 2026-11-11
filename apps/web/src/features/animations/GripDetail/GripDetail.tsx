import { Button, InlineBanner, Loader, cn } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { GripPicture } from '@/features/common/GripPicture';
import type { LibraryGripDoc, LibraryGripSummary } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { whenOf } from '@/state/plan-log';
import { gripBoneGroups } from '@/state/grip-bones';
import { handsWords, weaponLabel } from '@/state/recipe-grips';
import { gripLine, gripRevisionSource } from '@/state/recipe-library';
import { useGripDetail } from './GripDetail.hooks';

export interface GripDetailProps {
  summary: LibraryGripSummary;
}

const CAPTION = 'text-caption tracking-[0.04em] text-fg-mute uppercase';

/** What a revision holds: its picture, weapon, hands and bones by group. */
function Holds({ doc }: { doc: LibraryGripDoc }) {
  const { grip } = doc;
  const groups = gripBoneGroups(Object.keys(grip.pose), grip.hands.holds).map(g => (
    <li key={g.id} className="flex justify-between gap-2">
      <span>{g.label}</span>
      <span className="font-mono text-fg-mute">{g.bones.length}</span>
    </li>
  ));
  return (
    <>
      <div className="grid h-[150px] place-items-center rounded-card border border-line bg-studio p-3">
        <GripPicture
          pose={grip.pose}
          holds={grip.hands.holds}
          label={`r${doc.revision} of ${grip.name}, ${handsWords(grip)}`}
        />
      </div>
      <p className="text-meta text-fg-dim">
        Holds a {weaponLabel(grip.weapon).toLowerCase()} with {handsWords(grip)}
        {grip.hands.supports ? `, the ${grip.hands.holds} hand holding` : ''}.
      </p>
      <p className={CAPTION}>Bones</p>
      <ul
        aria-label="Bones"
        className="m-0 flex list-none flex-col gap-1 p-0 text-meta text-fg-dim"
      >
        {groups}
      </ul>
    </>
  );
}

/**
 * The selected library grip: every revision newest first with where it
 * came from, what the picked one holds, and the removals, each asked first.
 * A recipe takes a grip from its editor's grip picker.
 */
export default function GripDetail({ summary }: GripDetailProps) {
  const d = useGripDetail(summary);
  const now = clock.now();
  const revisions = d.revisions.map(r => (
    <li key={r.revision}>
      <button
        type="button"
        aria-pressed={r.revision === d.revision}
        onClick={() => d.pick(r.revision)}
        className={cn(
          'flex w-full cursor-pointer items-baseline gap-2 rounded-control px-2 py-1.5 text-left hover:bg-hover',
          r.revision === d.revision && 'bg-clay-soft hover:bg-clay-soft'
        )}
      >
        <b className="w-7 flex-none font-mono text-[12px] font-semibold">r{r.revision}</b>
        <span className="min-w-0 flex-1 truncate text-meta text-fg-dim">
          {gripRevisionSource(r, d.projectName)}
        </span>
        <span className="flex-none text-meta text-fg-mute">{whenOf(r.savedAt, now)}</span>
      </button>
    </li>
  ));
  const one = summary.revisions.length === 1;
  return (
    <aside
      aria-label="Selected grip"
      className="flex w-[320px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-[18px] min-[1280px]:w-[360px]"
    >
      <div>
        <h2 className="truncate text-panel-title">{summary.name}</h2>
        {summary.problem ? null : (
          <p className="mt-0.5 font-mono text-[11.5px] text-fg-mute">{gripLine(summary)}</p>
        )}
      </div>
      {summary.problem ? (
        <InlineBanner tone="danger" title="It could not be read" detail={summary.problem} />
      ) : null}
      {summary.latest > 0 ? (
        <section aria-label={`What r${d.revision} holds`} className="flex flex-col gap-1.5">
          {d.doc ? (
            <Holds doc={d.doc} />
          ) : d.error ? (
            <p role="alert" className="text-meta text-danger">
              {d.error}
            </p>
          ) : (
            <Loader label={`Reading r${d.revision}...`} />
          )}
        </section>
      ) : null}
      <section aria-label="Revisions" className="flex flex-col gap-1.5">
        <p className={CAPTION}>Revisions</p>
        <ul className="m-0 flex list-none flex-col gap-0.5 p-0">{revisions}</ul>
      </section>
      <p className="text-meta text-fg-mute">
        A recipe takes a grip from Add a grip in its editor, as its own copy with a link back.
      </p>
      {d.refused ? <InlineBanner tone="danger" title="Not removed" detail={d.refused} /> : null}
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="ghost"
          icon="trash"
          disabled={summary.latest === 0}
          tooltip={
            one
              ? 'The only revision: remove the whole grip instead'
              : `Moves r${d.revision} aside; the other revisions stay`
          }
          onClick={() => d.ask('revision')}
        >
          Remove r{d.revision}
        </Button>
        <Button size="sm" variant="ghost" icon="trash" onClick={() => d.ask('grip')}>
          Remove grip
        </Button>
      </div>
      {d.asking ? (
        <ChoiceDialog
          title={d.asking === 'grip' ? `Remove ${summary.name}?` : `Remove r${d.revision}?`}
          onDismiss={() => d.ask(null)}
          choices={[
            { label: 'Cancel', onClick: () => d.ask(null) },
            {
              label: d.asking === 'grip' ? 'Remove grip' : `Remove r${d.revision}`,
              variant: 'danger',
              loading: d.removing,
              onClick: d.remove,
            },
          ]}
        >
          <p>
            {d.asking === 'grip'
              ? 'Every revision is moved aside. Recipes that use it keep their copy, and say their library grip is gone.'
              : `r${d.revision} of ${summary.name} is moved aside. Recipes that use it keep their copy.`}
          </p>
        </ChoiceDialog>
      ) : null}
    </aside>
  );
}
