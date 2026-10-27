import type { ReactNode } from 'react';
import type { ButtonVariant } from '@maquette/ui';

export interface Choice {
  label: string;
  onClick: () => void;
  variant?: ButtonVariant;
  loading?: boolean;
  /** Greyed out, with `disabledReason` on hover. */
  disabled?: boolean;
  disabledReason?: string;
}

export interface ChoiceDialogProps {
  title: string;
  children: ReactNode;
  choices: Choice[];
  onDismiss: () => void;
}
