/**
 * Weak ref warnings above a send: an image too small, an object cropped at
 * the frame, a busy background, one view with multi-view on, a side or the
 * back that does not match the front. They never block the send; each
 * offers its one click fixes.
 */
import { Button, Icon } from '@maquette/ui';
import type { RefFix, RefWarning } from '@/ipc/client';
import { FIX_LABEL } from '@/state/refs';

export interface RefWarningsProps {
  warnings: readonly RefWarning[];
  onFix: (warning: RefWarning, fix: RefFix) => void;
}

export default function RefWarnings({ warnings, onFix }: RefWarningsProps) {
  const rows = warnings.map(w => {
    const fixes = w.fixes.map(fix => (
      <Button key={fix} size="sm" onClick={() => onFix(w, fix)}>
        {FIX_LABEL[fix]}
      </Button>
    ));
    return (
      <li
        key={`${w.kind}-${w.file ?? 'send'}`}
        className="flex items-start gap-2.5 rounded-card border border-amber/40 bg-amber-soft/40 px-3 py-2.5"
      >
        <Icon name="warning" size={16} className="mt-0.5 flex-none text-amber" />
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <div>
            <div className="font-semibold">{w.title}</div>
            <p className="text-meta text-fg-dim">{w.detail}</p>
          </div>
          <div className="flex flex-wrap gap-1.5">{fixes}</div>
        </div>
      </li>
    );
  });
  if (rows.length === 0) return null;
  return (
    <ul aria-label="Ref warnings" className="m-0 flex list-none flex-col gap-2 p-0">
      {rows}
    </ul>
  );
}
