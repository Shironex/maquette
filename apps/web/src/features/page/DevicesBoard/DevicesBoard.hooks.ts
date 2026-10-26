import { DEVICES, frameSource } from '@/state/page';
import { usePageStore } from '@/state/use-page-store';

/** One frame per device, of the same page and stop. */
export function useDevicesBoard() {
  const view = usePageStore(s => s.view);
  const poster = usePageStore(s => s.poster);
  const showStops = usePageStore(s => s.showStops);
  const readings = usePageStore(s => s.readings);
  const source = frameSource(view, { poster });
  return {
    src: source.src,
    address: source.address,
    badge:
      view?.settings.source === 'concept'
        ? ('concept' as const)
        : poster
          ? ('poster' as const)
          : ('live' as const),
    showStops,
    devices: DEVICES.map(d => {
      const stats = readings[d.id]?.stats;
      return {
        ...d,
        size: `${d.width} × ${d.height}`,
        frame: stats ? `p95 ${stats.p95Ms.toFixed(1)} ms` : 'no frame time yet',
      };
    }),
  };
}
