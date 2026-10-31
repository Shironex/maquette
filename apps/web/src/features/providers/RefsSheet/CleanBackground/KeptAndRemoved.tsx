import { Icon } from '@maquette/ui';
import type { CleanGround } from '@/ipc/client';
import { keptAndRemoved } from './keptRemoved';

export interface KeptAndRemovedProps {
  ground: CleanGround;
  pad: boolean;
  extend: boolean;
}

function List({ title, items, kept }: { title: string; items: string[]; kept: boolean }) {
  const rows = items.map(text => (
    <li key={text} className="flex items-start gap-2">
      <Icon
        name={kept ? 'check-circle' : 'x-circle'}
        size={14}
        className={kept ? 'mt-0.5 flex-none text-sage' : 'mt-0.5 flex-none text-fg-mute'}
      />
      <span>{text}</span>
    </li>
  ));
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1.5">
      <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
        {title}
      </h3>
      <ul className="flex list-none flex-col gap-1">{rows}</ul>
    </div>
  );
}

/** What stays and what goes in the copy, next to its options. */
export default function KeptAndRemoved({ ground, pad, extend }: KeptAndRemovedProps) {
  const { kept, removed } = keptAndRemoved(ground, pad, extend);
  return (
    <section aria-label="Kept and removed" className="flex flex-col gap-2.5">
      <div className="flex gap-4 rounded-card border border-line bg-raised px-3.5 py-3 text-meta">
        <List title="Kept" items={kept} kept />
        <List title="Removed" items={removed} kept={false} />
      </div>
    </section>
  );
}
