/**
 * A note above a send when the asset has a model already: the send makes
 * its next version, and a library entry it came from is not changed. It
 * never blocks the send.
 */
import { Icon } from '@maquette/ui';
import type { RefWarning } from '@/ipc/client';

export interface ModelNoteProps {
  note: RefWarning | null;
}

export default function ModelNote({ note }: ModelNoteProps) {
  if (!note) return null;
  return (
    <div
      role="note"
      aria-label="Existing model"
      className="flex items-start gap-2.5 rounded-card border border-amber/40 bg-amber-soft/40 px-3 py-2.5"
    >
      <Icon name="warning" size={16} className="mt-0.5 flex-none text-amber" />
      <div className="min-w-0">
        <div className="font-semibold">{note.title}</div>
        <p className="text-meta text-fg-dim">{note.detail}</p>
      </div>
    </div>
  );
}
