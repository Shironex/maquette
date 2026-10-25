import type { ViewId } from '@/state/views';

export interface InvalidSettingsDialogProps {
  /** The view the user asked for while a field is not valid. */
  leaveTo: ViewId;
  problems: readonly string[];
  /** Puts the invalid fields back to what is saved and keeps the rest. */
  discardInvalid: () => void;
}
