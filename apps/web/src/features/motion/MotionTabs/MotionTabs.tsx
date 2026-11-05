import { Tabs } from '@maquette/ui';
import { useMotionTabs } from './MotionTabs.hooks';

const TABS = [
  { id: 'clips', label: 'Clips' },
  { id: 'states', label: 'States' },
  { id: 'recipes', label: 'Recipes' },
];

/**
 * Clips or States for the selected model, and the project's Recipes. A game
 * has no States: states are moved by the website's stops, scroll and clicks,
 * which a game does not have.
 */
export default function MotionTabs() {
  const { tab, setTab, game } = useMotionTabs();
  return (
    <Tabs tabs={game ? TABS.filter(t => t.id !== 'states') : TABS} value={tab} onChange={setTab} />
  );
}
