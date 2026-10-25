import { Button } from '@maquette/ui';
import { useChoiceDialog } from './ChoiceDialog.hooks';
import type { ChoiceDialogProps } from './ChoiceDialog.types';

/**
 * A small question over the screen with two or three answers. Escape and a
 * backdrop click give `onDismiss`, the answer that changes nothing. The
 * last choice is the one the focus starts on.
 */
export default function ChoiceDialog({ title, children, choices, onDismiss }: ChoiceDialogProps) {
  const ref = useChoiceDialog(onDismiss);
  const busy = choices.some(c => c.loading);
  const last = choices.length - 1;
  const buttons = choices.map((choice, i) => (
    <Button
      key={choice.label}
      variant={choice.variant ?? 'secondary'}
      loading={choice.loading}
      disabled={busy}
      autoFocus={i === last}
      onClick={choice.onClick}
    >
      {choice.label}
    </Button>
  ));
  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget && !busy) onDismiss();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="choice-title"
        className="flex w-[420px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <div className="flex flex-col gap-1.5 px-5 pt-[18px] pb-4">
          <h2 id="choice-title" className="text-panel-title">
            {title}
          </h2>
          <div className="text-fg-dim">{children}</div>
        </div>
        <footer className="flex flex-wrap items-center justify-end gap-2 border-t border-line px-5 py-3">
          {buttons}
        </footer>
      </div>
    </div>
  );
}
