import type { KeyValueItem } from '@maquette/ui';
import type { Clip } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { isOff, n } from '@/state/motion';
import { currentClip, useMotionStore } from '@/state/use-motion-store';

/** "Root motion, 1.42 m", "In place" or unknown for a packed clip. */
export function rootText(clip: Clip): string {
  if (clip.loops === null && clip.rootTravel === null) return 'Not readable, packed';
  const travel = n(clip.rootTravel);
  return travel > 0 ? `Root motion, ${travel.toFixed(2)} m` : 'In place';
}

export function loopText(clip: Clip): string {
  if (clip.loops === null) return 'Not readable, packed';
  return clip.loops ? 'Ends where it starts' : 'Ends on another pose';
}

/** The facts of the selected clip, as its file has them. */
export function useClipDetails() {
  const clip = useMotionStore(currentClip);
  const asset = useMotionStore(s => s.asset);
  const draft = useMotionStore(s => s.draft);
  if (!clip || !asset) return null;
  const shipped = clip.shippedBytes;
  const items: KeyValueItem[] = [
    { label: 'Length', value: `${clip.frames} f · ${n(clip.duration).toFixed(2)} s`, mono: true },
    { label: 'Rate', value: `${n(clip.fps)} fps`, mono: true },
    { label: 'Keys', value: String(clip.keys), mono: true },
    { label: 'Channels', value: `${clip.channels} on ${clip.targets} nodes`, mono: true },
    { label: 'Root', value: rootText(clip) },
    { label: 'Loop', value: loopText(clip) },
    { label: 'In the source', value: formatBytes(n(clip.bytes)), mono: true },
    {
      label: 'On the site',
      value: shipped === null ? 'After the next run' : formatBytes(shipped),
      mono: true,
    },
  ];
  return { clip, items, off: isOff(draft, asset, clip.name) };
}
