import { Button, Icon } from '@maquette/ui';
import { LibraryDialogFrame } from '../LibraryDialogFrame';
import { useLibraryUpdateDialog } from './LibraryUpdateDialog.hooks';

/**
 * Update from library: what the newer library version brings in, which of
 * the asset's settings carry over and which no longer apply, with why.
 * Nothing changes until Update; the old versions stay in the project.
 */
export default function LibraryUpdateDialog() {
  const { plan, planError, busy, closeUpdate, update } = useLibraryUpdateDialog();
  const kept = (plan?.kept ?? []).map(k => (
    <li key={k} className="flex items-start gap-2">
      <Icon name="check" size={14} className="mt-0.5 flex-none text-sage" />
      <span>{k}</span>
    </li>
  ));
  const dropped = (plan?.dropped ?? []).map(d => (
    <li key={d.what} className="flex items-start gap-2">
      <Icon name="minus" size={14} className="mt-0.5 flex-none text-clay-text" />
      <span>
        <b className="font-semibold">{d.what}</b> no longer applies: {d.why}.
      </span>
    </li>
  ));

  return (
    <LibraryDialogFrame
      id="library-update-title"
      title={plan ? `Update ${plan.name} to library v${plan.to}` : 'Update from library'}
      help={
        plan
          ? `It is on library v${plan.from}. The new model comes in as v${plan.nextVersion} of the asset; its older versions stay.`
          : 'Reading what the update would change...'
      }
      onClose={closeUpdate}
      onSubmit={() => void update()}
      footer={
        <>
          <Button variant="ghost" onClick={closeUpdate}>
            Keep v{plan?.from ?? ''}
          </Button>
          <Button
            type="submit"
            variant="primary"
            icon="arrow-up"
            loading={busy}
            disabled={!plan || busy}
          >
            Update, no credits
          </Button>
        </>
      }
    >
      {kept.length ? (
        <div className="flex flex-col gap-1.5">
          <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
            Carries over
          </h3>
          <ul className="flex list-none flex-col gap-1 text-fg-dim">{kept}</ul>
        </div>
      ) : plan ? (
        <p className="text-meta text-fg-mute">The asset has no settings to carry over.</p>
      ) : null}
      {dropped.length ? (
        <div className="flex flex-col gap-1.5">
          <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
            Taken off
          </h3>
          <ul className="flex list-none flex-col gap-1 text-fg-dim">{dropped}</ul>
        </div>
      ) : null}
      {plan?.note ? <p className="text-meta text-fg-mute">{plan.note}</p> : null}
      {planError ? (
        <p role="alert" className="text-meta text-danger">
          {planError}
        </p>
      ) : null}
    </LibraryDialogFrame>
  );
}
