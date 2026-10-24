/** The segmented control choices the Send to 3D panels share. */
import type { Provider, ProviderId } from '@/ipc/client';
import type { SendDraft } from '@/state/providers';

export const PROVIDERS: { value: Provider; label: string }[] = [
  { value: 'tripo', label: 'Tripo' },
  { value: 'meshy', label: 'Meshy' },
  { value: 'by-hand', label: 'By hand' },
];

export const SERVICES: { value: ProviderId; label: string }[] = [
  { value: 'tripo', label: 'Tripo' },
  { value: 'meshy', label: 'Meshy' },
];

export const VARIANTS: { value: SendDraft['variant']; label: string }[] = [
  { value: 'prop', label: 'Prop' },
  { value: 'character', label: 'Character' },
];
