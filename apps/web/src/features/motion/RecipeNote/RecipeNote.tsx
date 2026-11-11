import type { ReactNode } from 'react';
import { Icon, cn } from '@maquette/ui';

export interface RecipeNoteProps {
  icon: 'check-circle' | 'warning' | 'info';
  title: string;
  detail: ReactNode;
  /** Amber, for what the person may want to act on. */
  warn?: boolean;
  /** A button or two under the detail. */
  actions?: ReactNode;
}

/** A note box under a change's settings: a pin's check, a grip's newer revision. */
export default function RecipeNote({ icon, title, detail, warn, actions }: RecipeNoteProps) {
  return (
    <div
      className={cn(
        'rounded-card border px-2.5 py-2',
        warn ? 'border-amber/60 bg-amber/10' : 'border-line bg-panel'
      )}
    >
      <p className="flex items-center gap-1.5 text-[12.5px] font-medium">
        <Icon name={icon} size={14} className={warn ? 'text-amber' : 'text-fg-dim'} />
        {title}
      </p>
      <div className="mt-0.5 text-meta text-fg-mute">{detail}</div>
      {actions ? <div className="mt-1.5 flex flex-wrap gap-1.5">{actions}</div> : null}
    </div>
  );
}
