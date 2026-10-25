import type { CallOutcome } from '@/ipc/client';

export const OUTCOME: Record<CallOutcome, { label: string; className: string }> = {
  ok: { label: 'ok', className: 'text-sage' },
  waiting: { label: 'waiting', className: 'text-amber' },
  error: { label: 'error', className: 'text-danger' },
  refused: { label: 'refused', className: 'text-danger' },
};
