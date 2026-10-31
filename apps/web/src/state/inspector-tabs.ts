import type { InspectorTab } from './use-inspector-store';

export interface InspectorTabDef {
  id: InspectorTab;
  label: string;
}

const WEBSITE_TABS: readonly InspectorTabDef[] = [
  { id: 'compare', label: 'Compare' },
  { id: 'layout', label: 'Layout' },
  { id: 'paths', label: 'Paths' },
  { id: 'parts', label: 'Parts' },
  { id: 'render', label: 'Render' },
];

/** A game has no scene to lay out: it checks each model against its budget. */
const GAME_TABS: readonly InspectorTabDef[] = [
  { id: 'compare', label: 'Compare' },
  { id: 'budget', label: 'Budget' },
  { id: 'parts', label: 'Parts' },
  { id: 'render', label: 'Render' },
];

/** The tabs a project shows. */
export function inspectorTabs(game: boolean): readonly InspectorTabDef[] {
  return game ? GAME_TABS : WEBSITE_TABS;
}

/**
 * The tab a project opens for the stored one: the row of tabs and the page
 * under it both read this, so they cannot disagree. A tab the project does not
 * have, as after switching project, falls back to Compare.
 */
export function shownInspectorTab(tab: InspectorTab, game: boolean): InspectorTab {
  return inspectorTabs(game).some(t => t.id === tab) ? tab : 'compare';
}
