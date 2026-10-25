import { Tabs } from '@maquette/ui';
import type { InspectorTab } from '@/state/use-inspector-store';
import { useInspectorTabs } from './InspectorTabs.hooks';

const TABS = [
  { id: 'compare', label: 'Compare' },
  { id: 'layout', label: 'Layout' },
] as const;

/** Compare versions, or lay out the scene. */
export default function InspectorTabs() {
  const { tab, setTab } = useInspectorTabs();
  return (
    <Tabs
      tabs={TABS}
      value={tab}
      onChange={id => setTab(id as InspectorTab)}
      className="-mt-1 flex-none"
    />
  );
}
