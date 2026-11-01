import type { ReactNode } from 'react';
import { cn } from '@maquette/ui';
import type { GameTemplate, GameTemplateId } from '@/ipc/client';
import { budgetsWords, exportWords, templateSections } from '@/state/game-templates';

export interface TemplateCardsProps {
  templates: GameTemplate[];
  picked: GameTemplateId | null;
  onPick: (template: GameTemplate) => void;
  /** Grid columns of each section, a Tailwind class. */
  columns: string;
  /** A card ahead of the sections, such as "No template". */
  leading?: ReactNode;
  /** The card's padding and name size, for a tight dialog. */
  compact?: boolean;
}

export const templateCardClass = (on: boolean, compact = false) =>
  cn(
    'cursor-pointer rounded-card border bg-raised text-left transition-colors duration-120',
    compact ? 'px-2.5 py-2' : 'px-3 py-2.5',
    on ? 'border-clay shadow-[0_0_0_1px_var(--clay)]' : 'border-line hover:border-line-strong'
  );

/**
 * The game templates under their section heads. Each card shows its numbers (the format,
 * the textures and the triangles per folder), so they read without opening a tooltip.
 */
export default function TemplateCards({
  templates,
  picked,
  onPick,
  columns,
  leading,
  compact = false,
}: TemplateCardsProps) {
  const card = (t: GameTemplate) => (
    <button
      key={t.id}
      type="button"
      aria-pressed={t.id === picked}
      title={t.why}
      className={templateCardClass(t.id === picked, compact)}
      onClick={() => onPick(t)}
    >
      <span className={cn('block font-semibold', compact ? 'text-[12.5px]' : 'text-[13px]')}>
        {t.name}
      </span>
      <span className="block text-meta text-fg-mute">{t.detail}</span>
      <span className="mt-1 block font-mono text-[11px] leading-4 text-fg-dim">
        {exportWords(t)}
      </span>
      <span className="block font-mono text-[11px] leading-4 text-fg-dim">{budgetsWords(t)}</span>
    </button>
  );
  const sections = templateSections(templates).map(section => {
    const cards = section.templates.map(card);
    return (
      <section
        key={section.id}
        role="group"
        aria-label={section.label}
        className="flex flex-col gap-1.5"
      >
        <h3 className="m-0 text-[12px] leading-4 font-medium text-fg-dim">{section.label}</h3>
        <div className={cn('grid gap-2', columns)}>{cards}</div>
      </section>
    );
  });
  return (
    <div className="flex flex-col gap-3">
      {leading ? <div className={cn('grid gap-2', columns)}>{leading}</div> : null}
      {sections}
    </div>
  );
}
