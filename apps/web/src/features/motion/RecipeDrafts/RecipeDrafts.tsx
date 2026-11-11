import { Icon } from '@maquette/ui';
import { useRecipeDrafts } from './RecipeDrafts.hooks';

/**
 * Says what an agent changed in the open recipe since a person saved it:
 * in the recipe already and baking as it is, shown in the agent's blue in
 * the changes and the key lanes until Save keeps it as the person's.
 */
export default function RecipeDrafts() {
  const d = useRecipeDrafts();
  if (!d) return null;
  return (
    <div
      role="status"
      aria-label="Agent drafts"
      className="flex items-start gap-2.5 rounded-card border border-agent/40 bg-agent/10 px-3 py-2"
    >
      <Icon name="robot" size={16} className="mt-0.5 flex-none text-agent" />
      <div className="min-w-0 flex-1">
        <b className="block text-[13px] font-semibold text-fg">{d.title}</b>
        <p className="text-meta text-fg-dim">{d.detail}</p>
      </div>
      <span className="flex flex-none items-center gap-1.5 text-meta text-fg-mute">
        <i aria-hidden className="size-2.5 rotate-45 rounded-[2px] bg-agent" />
        {d.legend}
      </span>
    </div>
  );
}
