import { Fragment } from 'react';
import { IconButton, SegmentedControl, TextField, cn } from '@maquette/ui';
import type { MeshySplit, SplitAfter, SplitSource } from '@/ipc/client';
import { MAX_MESHY_PARTS, MAX_RETEXTURE_PROMPT, MESHY_MODES, sourceLabel } from '@/state/split';
import { SectionTitle } from '../SectionTitle';
import { usePartNames } from './MeshyFields.hooks';

export interface MeshyFieldsProps {
  source: SplitSource;
  value: MeshySplit;
  onChange: (patch: Partial<MeshySplit>) => void;
  texture: TextureChoice;
}

/** What follows for the texture Meshy drops. */
export interface TextureChoice {
  after: SplitAfter;
  setAfter: (after: SplitAfter) => void;
  prompt: string;
  setPrompt: (prompt: string) => void;
}

/** Meshy Auto Split's mode, the part names it looks for, and what follows for the texture. */
export default function MeshyFields({ source, value, onChange, texture }: MeshyFieldsProps) {
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
      <section className="flex flex-col gap-2.5">
        <SectionTitle aside="after the parts arrive">Then</SectionTitle>
        {source.textures > 0 ? (
          <>
            <p className="text-meta text-fg-mute">
              Meshy drops the texture: the parts come back as grey geometry. {sourceLabel(source)}{' '}
              keeps its texture whatever you pick.
            </p>
            <AfterChoice source={source} texture={texture} />
          </>
        ) : (
          <p className="text-meta text-fg-mute">
            {sourceLabel(source)} has no texture, so the parts come back as they are.
          </p>
        )}
      </section>
    </>
  );
}

/** The three things that can happen to the texture, one to pick. */
function AfterChoice({
  source,
  texture: { after, setAfter, prompt, setPrompt },
}: Pick<MeshyFieldsProps, 'source' | 'texture'>) {
  const from = sourceLabel(source);
  const choices: { value: SplitAfter; title: string; tag?: string; body: string }[] = [
    {
      value: 'bake',
      title: 'Bake the texture back',
      tag: 'free',
      body: `Headless in Blender: projects ${from}'s texture onto the parts. Best when the split keeps the shape.`,
    },
    {
      value: 'retexture',
      title: 'Re-texture with Meshy',
      tag: 'more credits',
      body: 'Paints the model again from the texture prompt, for a new look. Meshy may merge the parts into one mesh; the bake back keeps them. You start it from the Parts tab after accepting the parts, and it asks before it spends.',
    },
    {
      value: 'grey',
      title: 'Keep it grey for now',
      body: 'Set up Motion states first and texture later. The bake back and the re-texture stay on offer.',
    },
  ];
  const cards = choices.map(c => (
    <Fragment key={c.value}>
      <label
        className={cn(
          'flex cursor-pointer items-start gap-2.5 rounded-card border p-3',
          after === c.value ? 'border-clay-line bg-raised' : 'border-line hover:border-line-strong'
        )}
      >
        <input
          type="radio"
          name="split-after"
          className="mt-0.5 accent-clay"
          checked={after === c.value}
          onChange={() => setAfter(c.value)}
        />
        <span className="flex min-w-0 flex-col gap-0.5">
          <span className="flex items-baseline justify-between gap-2 font-semibold">
            {c.title}
            {c.tag ? <span className="text-meta font-normal text-fg-mute">{c.tag}</span> : null}
          </span>
          <span className="text-meta text-fg-mute">{c.body}</span>
        </span>
      </label>
      {c.value === 'retexture' && after === 'retexture' ? (
        <TextField
          aria-label="Texture prompt"
          placeholder="Weathered iron, frosted glass, warm glow"
          maxLength={MAX_RETEXTURE_PROMPT}
          value={prompt}
          onChange={e => setPrompt(e.target.value)}
        />
      ) : null}
    </Fragment>
  ));
  return (
    <div role="radiogroup" aria-label="After the parts arrive" className="flex flex-col gap-2">
      {cards}
    </div>
  );
}
