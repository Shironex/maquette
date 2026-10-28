import { Button, Icon, IconButton, cn } from '@maquette/ui';
import { STEPS } from '@/state/retarget';
import { ChooseStep } from '../ChooseStep';
import { MapStep } from '../MapStep';
import { ReviewStep } from '../ReviewStep';
import { SettingsStep } from '../SettingsStep';
import { useRetargetDialog } from './RetargetDialog.hooks';

const BODY = { choose: ChooseStep, map: MapStep, settings: SettingsStep, review: ReviewStep };

/**
 * Library clips onto a rigged model, in four steps: choose clips, check
 * the bone map, set each clip up, and review a headless bake before it
 * becomes the model's next version.
 */
export default function RetargetDialog() {
  const dialog = useRetargetDialog();
  if (!dialog) return null;
  const { sheet, primary } = dialog;
  const Body = BODY[sheet.step];
  const steps = STEPS.map((step, i) => {
    const done = i < dialog.at;
    const open = i === 0 || (dialog.canJump && i < 3) || (i === 3 && Boolean(sheet.draft));
    return (
      <li key={step.id}>
        <button
          type="button"
          aria-current={i === dialog.at ? 'step' : undefined}
          disabled={!open || dialog.busy}
          onClick={() => dialog.goTo(step.id)}
          className={cn(
            'flex h-8 cursor-pointer items-center gap-2 rounded-pill px-3 text-[12.5px] text-fg-mute disabled:cursor-default',
            i === dialog.at && 'bg-clay-soft font-medium text-fg',
            done && 'text-fg-dim'
          )}
        >
          <span className="font-mono text-[11px]">
            {done ? <Icon name="check" size={11} /> : i + 1}
          </span>
          {step.label}
          {done ? <span className="sr-only">, done</span> : null}
        </button>
      </li>
    );
  });
  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) dialog.close();
      }}
    >
      <div
        ref={dialog.ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="retarget-title"
        className="flex h-[720px] max-h-full w-[1220px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <header className="flex items-start justify-between gap-4 border-b border-line px-5 pt-[18px] pb-3">
          <div className="min-w-0">
            <h2 id="retarget-title" className="text-[17px] leading-6 font-semibold">
              {dialog.title}
            </h2>
            <p className="text-meta text-fg-dim">{dialog.subtitle}</p>
          </div>
          <IconButton icon="x" label="Close" onClick={dialog.close} />
        </header>
        <nav aria-label="Steps" className="border-b border-line px-4 py-2">
          <ol className="flex flex-wrap gap-1">{steps}</ol>
        </nav>
        <div className="flex min-h-0 flex-1">
          <Body />
        </div>
        <footer className="flex flex-wrap items-center gap-2 border-t border-line px-5 py-3">
          {sheet.error ? (
            <p role="alert" className="mr-auto text-meta text-danger">
              {sheet.error}
            </p>
          ) : (
            <span className="mr-auto" />
          )}
          {sheet.step === 'choose' ? (
            <Button onClick={dialog.close}>Cancel</Button>
          ) : (
            <Button onClick={dialog.back} disabled={dialog.busy}>
              Back
            </Button>
          )}
          <Button
            variant="primary"
            onClick={() => void primary.onClick()}
            disabled={primary.disabled}
            disabledReason={primary.reason}
          >
            {primary.label}
          </Button>
        </footer>
      </div>
    </div>
  );
}
