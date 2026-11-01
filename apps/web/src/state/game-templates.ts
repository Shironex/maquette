import type { FolderBudget, GameTemplate, GameTemplateGroup } from '@/ipc/client';
import { formatK } from '@/lib/format';

/** The picker's sections, in order, with the words that head them. */
const GROUPS: { id: GameTemplateGroup; label: string }[] = [
  { id: 'shape', label: 'By shape of the models' },
  { id: 'engine', label: 'By engine' },
  { id: 'target', label: 'By device or kind of project' },
];

export interface TemplateSection {
  id: GameTemplateGroup;
  label: string;
  templates: GameTemplate[];
}

/** The templates under their section heads, empty sections left out. */
export function templateSections(templates: GameTemplate[]): TemplateSection[] {
  return GROUPS.map(g => ({ ...g, templates: templates.filter(t => t.group === g.id) })).filter(
    s => s.templates.length > 0
  );
}

/** `props/ 10k` for one budget row; the model folder itself is "all folders". */
export const budgetWords = (b: FolderBudget): string =>
  `${b.folder ? `${b.folder}/` : 'all folders'} ${formatK(b.triangles)}`;

/** What a template sets besides the budgets: `.glb, JPG up to 1024 px, no extra maps`. */
export function exportWords(t: GameTemplate): string {
  const parts = [`.${t.format}`];
  if (t.textures) {
    const cap = t.textureCap ? ` up to ${t.textureCap} px` : '';
    parts.push(`${t.textures === 'keep' ? 'Keep' : t.textures.toUpperCase()}${cap}`);
  } else if (t.textureCap) {
    parts.push(`textures up to ${t.textureCap} px`);
  }
  if (t.dropMaps) parts.push('no extra maps');
  return parts.join(', ');
}

/** The triangle budgets a template lays out: `props/ 10k, characters/ 25k, kit/ 3k`. */
export const budgetsWords = (t: GameTemplate): string => t.budgets.map(budgetWords).join(', ');
