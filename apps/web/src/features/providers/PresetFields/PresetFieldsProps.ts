import type { PresetVariant } from '@/ipc/client';

/** What the Tripo and Meshy preset fields take, for a preset of type `P`. */
export interface PresetFieldsProps<P> {
  preset: P;
  onChange: (preset: P) => void;
  variant: PresetVariant;
  error?: string;
  compact?: boolean;
}
