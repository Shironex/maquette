import { Button, InlineBanner, KeyValueList } from '@maquette/ui';
import type { AnimationItem } from '@/ipc/client';
import {
  RIG_LABEL,
  lengthAtLine,
  originLine,
  rootMotionLine,
  storedLine,
} from '@/state/animations';
import { ClipPreview } from '../ClipPreview';
import { useClipDetail } from './ClipDetail.hooks';

export interface ClipDetailProps {
  item: AnimationItem;
}

/**
 * The selected clip: playing on its own skeleton, its numbers, Use on
 * model, the models that baked it, and the license note for a Mixamo clip.
 */
export default function ClipDetail({ item }: ClipDetailProps) {
  const { clip } = item;
  const detail = useClipDetail(item);
  const uses = item.uses.map(use => (
    <li key={`${use.project}/${use.asset}`} className="flex items-center gap-2 text-meta">
      <span className="min-w-0 flex-1 truncate text-fg-dim">
        {use.assetName} v{use.version} · {use.projectName} ·{' '}
        <span className="font-mono">{use.name}</span>
      </span>
      {use.outdated ? (
        <Button
          size="sm"
          onClick={() => detail.update(use)}
          tooltip="Bakes the newer take into the model's next version"
        >
          Update
        </Button>
      ) : null}
    </li>
  ));
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
      <Button
        variant="primary"
        icon="arrows-left-right"
        onClick={detail.useOnModel}
        disabled={!detail.project}
        disabledReason="Open a project to put the clip on one of its models"
      >
        Use on model
      </Button>
      <section aria-label="Where it is used" className="flex flex-col gap-1.5">
        <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">Where it is used</p>
        {uses.length ? (
          <ul className="flex flex-col gap-1">{uses}</ul>
        ) : (
          <p className="text-meta text-fg-mute">No model has baked it yet.</p>
        )}
      </section>
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
