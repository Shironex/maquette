import { InlineBanner, KeyValueList } from '@maquette/ui';
import type { AnimationItem } from '@/ipc/client';
import {
  RIG_LABEL,
  lengthAtLine,
  originLine,
  rootMotionLine,
  storedLine,
} from '@/state/animations';
import { ClipPreview } from '../ClipPreview';

export interface ClipDetailProps {
  item: AnimationItem;
}

/**
 * The selected clip: playing on its own skeleton, its numbers, and the
 * license note for a Mixamo clip.
 */
export default function ClipDetail({ item }: ClipDetailProps) {
  const { clip } = item;
  const tags = clip.tags.map(tag => (
    <span
      key={tag}
      className="rounded-pill border border-line px-1.5 font-mono text-[11px] leading-[18px] text-fg-mute"
    >
      {tag}
    </span>
  ));
  const facts = [
    { label: 'Length', value: lengthAtLine(clip) },
    { label: 'Root motion', value: rootMotionLine(clip) },
    { label: 'Loop', value: clip.loops ? 'yes' : 'no' },
    { label: 'Source rig', value: `${RIG_LABEL[clip.rig.kind]} · ${clip.rig.bones.length} bones` },
    { label: 'Stored', value: storedLine(clip) },
  ];
  return (
    <aside
      aria-label="Selected clip"
      className="flex w-[360px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-[18px]"
    >
      <ClipPreview key={clip.id} item={item} className="aspect-[4/3] w-full flex-none" />
      <div>
        <h2 className="text-panel-title">{clip.name}</h2>
        <p className="mt-0.5 text-meta text-fg-mute">{originLine(clip)}</p>
        {tags.length ? <p className="mt-2 flex flex-wrap gap-1">{tags}</p> : null}
      </div>
      <KeyValueList items={facts} />
      {clip.revision > 1 ? (
        <p className="text-meta text-fg-mute">
          Take {clip.revision}: an import replaced the keys of an older take.
        </p>
      ) : null}
      {clip.rig.kind === 'mixamo' ? (
        <InlineBanner
          tone="info"
          title="Mixamo clip."
          detail="Free to use in your projects, not to share as a raw file. The library stays on this machine; clips leave it only baked into a model."
        />
      ) : null}
    </aside>
  );
}
