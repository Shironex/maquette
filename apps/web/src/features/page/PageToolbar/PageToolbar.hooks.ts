import { useState } from 'react';
import { DEVICES, siteName, type DeviceId } from '@/state/page';
import { usePageStore, type CompareMode } from '@/state/use-page-store';

export const MODES: readonly { value: CompareMode; label: string; hint: string }[] = [
  { value: 'one', label: 'One', hint: 'One page, with the scrubber' },
  { value: 'ship-run', label: 'Ship vs run', hint: 'The site now beside this run' },
  { value: 'poster', label: 'With poster', hint: 'Adds the page without WebGL' },
];

/** "Home page", "Pricing page", from the path on the site. */
export function pageTitle(path: string, concept: boolean): string {
  if (concept) return 'Page concept';
  const last = path.split('/').filter(Boolean).pop();
  if (!last) return 'Home page';
  const words = decodeURIComponent(last).replace(/[-_]+/g, ' ').trim();
  return `${words.charAt(0).toUpperCase()}${words.slice(1)} page`;
}

/** The title, the path field, the device and the overlays. */
export function usePageToolbar() {
  const view = usePageStore(s => s.view);
  const device = usePageStore(s => s.device);
  const mode = usePageStore(s => s.mode);
  const tab = usePageStore(s => s.tab);
  const showStops = usePageStore(s => s.showStops);
  const hotspots = usePageStore(s => s.hotspots);
  const poster = usePageStore(s => s.poster);
  const store = usePageStore.getState;
  const path = view?.settings.path ?? '/';
  const concept = view?.settings.source === 'concept';
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(path);
  return {
    title: pageTitle(path, concept),
    pathLabel: `${siteName(view?.siteRepo ?? null)} · ${path}`,
    concept,
    editing,
    draft,
    setDraft,
    editPath: () => {
      setDraft(path);
      setEditing(true);
    },
    savePath: async () => {
      const next = draft.trim() || '/';
      if (next === path || (await store().save({ path: next }))) setEditing(false);
    },
    cancelPath: () => setEditing(false),
    devices: DEVICES.map(d => ({ value: d.id, label: d.label, icon: d.icon })),
    device,
    setDevice: (id: DeviceId) => store().setDevice(id),
    mode,
    setMode: (next: CompareMode) => store().setMode(next),
    devicesTab: tab === 'devices',
    showStops,
    toggleStops: () => store().setShowStops(!showStops),
    hotspots: { on: hotspots, toggle: () => store().setHotspots(!hotspots) },
    poster,
    togglePoster: () => store().setPoster(!poster),
  };
}
