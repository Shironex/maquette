import { Tabs } from '@maquette/ui';
import { useMotionTabs } from './MotionTabs.hooks';

const TABS = [
  { id: 'clips', label: 'Clips' },
  { id: 'states', label: 'States' },
];

/**
 * Clips or States for the selected model. A game has only Clips: states are
 * moved by the website's stops, scroll and clicks, which a game does not have.
 */
export default function MotionTabs() {
  const { tab, setTab, game } = useMotionTabs();
  return <Tabs tabs={game ? TABS.slice(0, 1) : TABS} value={tab} onChange={setTab} />;
}
