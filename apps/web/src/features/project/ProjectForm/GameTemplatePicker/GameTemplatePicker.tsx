import { Icon, cn } from '@maquette/ui';
import type { GameTemplate } from '@/ipc/client';
import { formatK } from '@/lib/format';
import type { ProjectFields } from '@/state/onboarding';
import { useGameTemplateList } from '@/state/use-game-template-list';

export interface GameTemplatePickerProps {
  value: ProjectFields;
  onChange: (patch: Partial<ProjectFields>) => void;
}

const card = (on: boolean) =>
  cn(
    'cursor-pointer rounded-card border bg-raised px-2.5 py-2 text-left transition-colors duration-120',
    on ? 'border-clay shadow-[0_0_0_1px_var(--clay)]' : 'border-line hover:border-line-strong'
  );

/** What the picked template adds, in words. */
function summary(template: GameTemplate): string {
  const rows = template.budgets
    .map(b => `${b.folder}/ at ${formatK(b.triangles)} triangles, ${b.texture} px`)
    .join(', ');
  return `Export settings only: .${template.format} and ${rows}. Adds no models and no game code.`;
}

/** An optional template for a new game: the format and folder budgets it starts with. */
export default function GameTemplatePicker({ value, onChange }: GameTemplatePickerProps) {
  const templates = useGameTemplateList();
  const picked = templates.find(t => t.id === value.gameTemplate) ?? null;
  const cards = templates.map(t => (
    <button
      key={t.id}
      type="button"
      aria-pressed={t.id === picked?.id}
      title={t.why}
      className={card(t.id === picked?.id)}
      onClick={() => onChange({ gameTemplate: t.id, gameFormat: t.format })}
    >
      <span className="block text-[12.5px] font-semibold">{t.name}</span>
      <span className="block text-meta text-fg-mute">{t.detail}</span>
    </button>
  ));
  return (
    <div className="mt-4 flex flex-col gap-1.5">
      <div className="grid grid-cols-3 gap-2" role="group" aria-label="Game templates">
        <button
          type="button"
          aria-pressed={!picked}
          className={card(!picked)}
          onClick={() => onChange({ gameTemplate: null })}
        >
          <span className="block text-[12.5px] font-semibold">No template</span>
          <span className="block text-meta text-fg-mute">From the repo only</span>
        </button>
        {cards}
      </div>
      {picked ? (
        <p className="m-0 text-meta text-fg-mute">{summary(picked)}</p>
      ) : (
        <p className="m-0 text-meta text-fg-mute">
          A template only sets export settings. The game still starts from its own models.
        </p>
      )}
      {picked && picked.format !== value.gameFormat ? (
        <p className="m-0 flex items-center gap-1.5 text-meta text-amber">
          <Icon name="warning" size={14} />
          This template writes .{picked.format}, the Model format below says .{value.gameFormat}.
          The Model format below wins.
        </p>
      ) : null}
    </div>
  );
}
