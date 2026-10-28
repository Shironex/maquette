import { Tabs } from '@maquette/ui';
import type { InspectorTab } from '@/state/use-inspector-store';
import { useInspectorTabs } from './InspectorTabs.hooks';

const TABS = [
  { id: 'compare', label: 'Compare' },
  { id: 'layout', label: 'Layout' },
  { id: 'paths', label: 'Paths' },
  { id: 'parts', label: 'Parts' },
] as const;

/** A game has no scene to lay out: it checks each model against its budget. */
const GAME_TABS = [
  { id: 'compare', label: 'Compare' },
  { id: 'budget', label: 'Budget' },
  { id: 'parts', label: 'Parts' },
] as const;

/** Compare versions, lay out the scene, or draw paths through it; in a game, the budgets. */
export default function InspectorTabs() {
  const { tab, setTab, game } = useInspectorTabs();
  if (!game) {
    return (
      <Tabs
        tabs={TABS}
        value={tab}
        onChange={id => setTab(id as InspectorTab)}
        className="-mt-1 flex-none"
      />
    );
  }
  return (
    <div className="-mt-1 flex flex-none items-end gap-4 border-b border-line">
      <Tabs
        tabs={GAME_TABS}
        value={tab}
        onChange={id => setTab(id as InspectorTab)}
        className="border-b-0"
      />
      <span
        title="Game preview is planned: the model in the game's own lighting. Not built yet."
        className="inline-flex h-9 items-center gap-1.5 font-medium text-fg-mute opacity-70"
      >
        Game preview
        <span className="rounded-pill border border-line px-1.5 text-[11px] leading-4">later</span>
      </span>
    </div>
  );
}
