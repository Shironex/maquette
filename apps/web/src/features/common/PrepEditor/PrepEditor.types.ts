import type { FixupDefaults, ModelPrep, Suggestion } from '@/ipc/client';
import type { PreviewState } from '@/state/use-prepare-store';

/** Floats cross the IPC as `number | null`. */
type Sides = readonly [number | null, number | null, number | null];

export interface PrepEditorProps {
  value: ModelPrep;
  onChange: (next: ModelPrep) => void;
  /** Width, height and depth of the model as it came, metres. */
  measured: Sides | null;
  /** The same after the fix-ups, once a preview ran. */
  fixed?: Sides | null;
  suggestion: Suggestion | null;
  defaults: FixupDefaults;
  /** The preview of `value`, when one ran or runs. */
  preview: PreviewState | null;
  /** Runs `value` headless on the model; null where there is no model to run on. */
  onPreview: (() => void) | null;
  /** Why the preview cannot run, shown on its button. */
  previewBlocked?: string | null;
  /** What "Skip the bake" shows while nobody chose: on for an Inbox model. */
  skipBakeDefault?: boolean;
  disabled?: boolean;
}
