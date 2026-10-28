import { SegmentedControl, Select, Toggle } from '@maquette/ui';
import type { CompletionMode, SplitSource, TripoSplit } from '@/ipc/client';
import { GRANULARITIES, SEGMENT_MODELS, sourceLabel } from '@/state/split';
import { SectionTitle } from '../SectionTitle';
import { noteCard } from './noteCard';

const COMPLETION: readonly { value: CompletionMode; label: string }[] = [
  { value: 'ai-completion', label: 'AI completion' },
  { value: 'quick-cap', label: 'Quick cap' },
];

export interface TripoFieldsProps {
  source: SplitSource;
  value: TripoSplit;
  onChange: (patch: Partial<TripoSplit>) => void;
  /** The version the split lands as. */
  next: string;
}

/** Tripo's segmentation settings and what comes back. */
export default function TripoFields({ source, value, onChange, next }: TripoFieldsProps) {
  const semantic = value.model === 'semantic';
  return (
    <>
      <section className="flex flex-col gap-2.5">
        <SectionTitle aside={<code className="font-mono">POST /v3/mesh/segment</code>}>
          Tripo segment
        </SectionTitle>
        <SegmentedControl
          label="Model"
          tone="clay"
          block
          options={SEGMENT_MODELS}
          value={value.model}
          onChange={model => onChange({ model })}
        />
        <p className="text-meta text-fg-mute">
          Geometry cuts where the shape changes. Semantic looks for parts that mean something, like
          a lid or a cage.
        </p>
        {semantic ? (
          <>
            <SegmentedControl
              label="Granularity"
              tone="clay"
              block
              options={GRANULARITIES}
              value={value.granularity}
              onChange={granularity => onChange({ granularity })}
            />
            <p className="text-meta text-fg-mute">
              Simple gives a few big parts, Detailed many small ones.
            </p>
            <Toggle
              label="Split loose pieces too"
              description="Pieces that do not touch always become parts of their own."
              checked={value.splitByConnectivity}
              onChange={splitByConnectivity => onChange({ splitByConnectivity })}
            />
          </>
        ) : null}
        <Toggle
          label="Complete hidden parts"
          description="A second Tripo task rebuilds faces one part hid in another, like the post inside the cage. More credits."
          checked={value.complete !== null}
          onChange={on => onChange({ complete: on ? 'ai-completion' : null })}
        />
        {value.complete ? (
          <Select
            label="Completion"
            inline
            options={COMPLETION}
            value={value.complete}
            onChange={complete => onChange({ complete })}
          />
        ) : null}
      </section>
      <div className={noteCard}>
        <h4 className="font-semibold">What comes back</h4>
        <p className="text-meta text-fg-mute">
          One GLB, one node per part: <code className="font-mono">tripo_part_0</code>,{' '}
          <code className="font-mono">tripo_part_1</code> and on. You name them in the Inspector.
        </p>
        <h4 className="font-semibold">Lands in the Inbox as {next}</h4>
        <p className="text-meta text-fg-mute">
          {sourceLabel(source)} stays, so you can compare the two and keep one.
        </p>
        {source.textures > 0 ? (
          <>
            <h4 className="font-semibold">Texture: not confirmed</h4>
            <p className="text-meta text-fg-mute">
              Tripo does not say whether a split keeps the texture. The Inbox checks on arrival and
              offers a free bake back from {sourceLabel(source)} if it is gone.
            </p>
          </>
        ) : null}
      </div>
    </>
  );
}
