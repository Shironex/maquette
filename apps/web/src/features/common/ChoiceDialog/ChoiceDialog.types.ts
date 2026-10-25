import type { ReactNode } from 'react';
import type { ButtonVariant } from '@maquette/ui';

export interface Choice {
  label: string;
  onClick: () => void;
  variant?: ButtonVariant;
  loading?: boolean;
}

export interface ChoiceDialogProps {
  title: string;
  children: ReactNode;
  choices: Choice[];
  onDismiss: () => void;
}
