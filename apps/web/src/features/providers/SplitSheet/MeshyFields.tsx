import { IconButton, SegmentedControl, TextField } from '@maquette/ui';
import type { MeshySplit, SplitSource } from '@/ipc/client';
import { MAX_MESHY_PARTS, MESHY_MODES, sourceLabel } from '@/state/split';
import { SectionTitle } from '../SectionTitle';
import { usePartNames } from './MeshyFields.hooks';
import { noteCard } from './noteCard';

export interface MeshyFieldsProps {
  source: SplitSource;
  value: MeshySplit;
  onChange: (patch: Partial<MeshySplit>) => void;
}

/** Meshy Auto Split's mode, the part names it looks for, and the bake back after. */
export default function MeshyFields({ source, value, onChange }: MeshyFieldsProps) {
  const names = value.parts ?? [];
  const named = value.mode !== 'auto';
  const input = usePartNames(names, onChange);
  const chips = names.map(name => (
    <li
      key={name}
      className="inline-flex items-center gap-1 rounded-full border border-line bg-raised py-0.5 pr-0.5 pl-2.5 font-mono text-meta text-fg"
    >
      {name}
      <IconButton
        icon="x"
        size="sm"
        label={`Take out ${name}`}
        onClick={() => input.remove(name)}
      />
    </li>
  ));
  return (
    <>
      <section className="flex flex-col gap-2.5">
        <SectionTitle aside={<code className="font-mono">POST /openapi/v1/print/split</code>}>
          Meshy Auto Split
        </SectionTitle>
        <SegmentedControl
          label="Mode"
          tone="clay"
          block
          options={MESHY_MODES}
          value={value.mode}
          onChange={mode => onChange({ mode })}
        />
        {named ? (
          <>
            <SectionTitle aside={`${names.length} of ${MAX_MESHY_PARTS}`}>
              {value.mode === 'by-color' ? 'Colours to find' : 'Parts to find'}
            </SectionTitle>
            {names.length ? <ul className="flex flex-wrap gap-1.5">{chips}</ul> : null}
            <TextField
              aria-label="Add a part name"
              placeholder="lid, cage, crystal"
              value={input.typed}
              disabled={names.length >= MAX_MESHY_PARTS}
              onChange={e => input.setTyped(e.target.value)}
              onKeyDown={input.onKeyDown}
              onBlur={input.onBlur}
            />
            <p className="text-meta text-fg-mute">
              Name them the way you would say them. Meshy looks for each and returns one part per
              name.
            </p>
          </>
        ) : (
          <p className="text-meta text-fg-mute">
            Meshy chooses the cuts. Pick By parts to name what it should look for.
          </p>
        )}
      </section>
      <div className={noteCard}>
        <h4 className="font-semibold">Then, after the parts arrive</h4>
        {source.textures > 0 ? (
          <p className="text-meta text-fg-mute">
            Meshy drops the texture: the parts come back as grey geometry. Once you accept them, the
            texture bakes back from {sourceLabel(source)} in Blender, free and headless, as the
            version after. The grey version stays, so you can keep it instead.
          </p>
        ) : (
          <p className="text-meta text-fg-mute">
            {sourceLabel(source)} has no texture, so the parts come back as they are.
          </p>
        )}
        <p className="text-meta text-fg-mute">
          {sourceLabel(source)} keeps its texture whatever happens.
        </p>
      </div>
    </>
  );
}
