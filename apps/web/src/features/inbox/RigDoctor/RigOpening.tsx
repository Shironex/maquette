import { Icon, Loader, TopBar } from '@maquette/ui';
import { usePastRun } from '@/lib/waiting';
import { RIG_OPEN_WAIT } from '@/state/use-rig-store';
import { useRigCrumbs } from './RigOpening.hooks';

/** Where the case sits: the view it came from, the model, the rig doctor. */
export function RigCrumbs({ name }: { name: string }) {
  const { leave, back } = useRigCrumbs();
  return (
    <nav aria-label="Where you are" className="flex items-center gap-2 text-fg-mute">
      <button
        type="button"
        className="cursor-pointer rounded-control hover:text-fg hover:underline"
        title={`Back to the ${back}. A run goes on.`}
        onClick={leave}
      >
        {back}
      </button>
      <Icon name="caret-right" size={11} />
      <span className="max-w-[28ch] truncate">{name}</span>
      <Icon name="caret-right" size={11} />
      <h1 className="m-0 text-panel-title text-fg">Rig doctor</h1>
    </nav>
  );
}

/** Shown at once while the rig doctor reads a model's rig, so the click is answered. */
export default function RigOpening({ name }: { name: string }) {
  const estimate = usePastRun(RIG_OPEN_WAIT);
  return (
    <>
      <TopBar lead={<RigCrumbs name={name} />} />
      <main className="grid min-h-0 flex-1 place-items-center" aria-busy="true">
        <Loader label={`Reading the rig of ${name}...`} estimate={estimate} />
      </main>
    </>
  );
}
