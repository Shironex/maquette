import { Icon } from '@maquette/ui';
import type { GameTemplate } from '@/ipc/client';
import { TemplateCards, templateCardClass } from '@/features/common/TemplateCards';
import { budgetWords, exportWords } from '@/state/game-templates';
import type { ProjectFields } from '@/state/onboarding';
import { useGameTemplateList } from '@/state/use-game-template-list';

export interface GameTemplatePickerProps {
  value: ProjectFields;
  onChange: (patch: Partial<ProjectFields>) => void;
}

/** What the picked template adds, in words. */
function summary(template: GameTemplate): string {
  const rows = template.budgets.map(b => `${budgetWords(b)} triangles, ${b.texture} px`).join(', ');
  return `Export settings only: ${exportWords(template)}, and ${rows}. Adds no models and no game code.`;
}

/** An optional template for a new game: the format, textures and folder budgets it starts with. */
export default function GameTemplatePicker({ value, onChange }: GameTemplatePickerProps) {
  const templates = useGameTemplateList();
  const picked = templates.find(t => t.id === value.gameTemplate) ?? null;
  const none = (
    <button
      type="button"
      aria-pressed={!picked}
      className={templateCardClass(!picked, true)}
      onClick={() => onChange({ gameTemplate: null })}
    >
      <span className="block text-[12.5px] font-semibold">No template</span>
      <span className="block text-meta text-fg-mute">From the repo only</span>
    </button>
  );
  return (
    <div className="mt-4 flex flex-col gap-1.5">
      <TemplateCards
        templates={templates}
        picked={picked?.id ?? null}
        onPick={t => onChange({ gameTemplate: t.id, gameFormat: t.format })}
        columns="grid-cols-3"
        leading={none}
        compact
      />
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
