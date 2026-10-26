import { Tabs } from '@maquette/ui';
import { useMotionTabs } from './MotionTabs.hooks';

const TABS = [
  { id: 'clips', label: 'Clips' },
  { id: 'states', label: 'States' },
];

/** Clips or States for the selected model. */
export default function MotionTabs() {
  const { tab, setTab } = useMotionTabs();
  return <Tabs tabs={TABS} value={tab} onChange={setTab} />;
}
