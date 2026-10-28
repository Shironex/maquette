import { Button, SegmentedControl, Select, TextField, cn } from '@maquette/ui';
import { SectionTitle } from '../SectionTitle';
import { Sheet } from '@/features/common/Sheet';
import { MAX_ASSEMBLE_PARTS } from '@/state/split';
import AssemblePartRow from './AssemblePartRow';
import { useAssembleSheet, type Placement } from './AssembleSheet.hooks';

const PLACEMENTS: readonly { value: Placement; label: string; hint: string }[] = [
  {
    value: 'model',
    label: 'Where the models put them',
    hint: 'Each part sits where its model was made to sit, like parts made from one concept.',
  },
  { value: 'numbers', label: 'By numbers', hint: 'Place each pivot in metres, Y up.' },
];

/** Assemble one prop from part models: one GLB, a node per part under one root. */
export default function AssembleSheet() {
  const a = useAssembleSheet();
  if (!a.target) return null;
  const nameOf = (id: string) => a.models.find(m => m.id === id)?.name ?? id;
  const prop = a.name.trim() || 'prop';
  const intoOptions = [
    { value: '', label: `A new prop, ${prop}` },
    ...a.models.map(m => ({ value: m.id, label: `${m.name}, as its next version` })),
  ];
  const last = a.parts.length - 1;
  const tree = a.parts.map((p, i) => (
    <li key={p.asset} className="font-mono text-meta">
      <span className="text-fg-mute">{i === last ? '└ ' : '├ '}</span>
      <span className="text-fg">{p.name.trim() || '?'}</span>
      <span className="text-fg-mute"> · {p.pivot.replace('-', ' ')}</span>
    </li>
  ));
  const rows = a.parts.map((part, i) => (
    <AssemblePartRow
      key={part.asset}
      part={part}
      index={i}
      count={a.parts.length}
      assetName={nameOf(part.asset)}
      byNumbers={a.placement === 'numbers'}
      onChange={patch => a.update(i, patch)}
      onMove={by => a.move(i, by)}
      onRemove={() => a.remove(i)}
    />
  ));
  const addOptions = [
    { value: '', label: 'Add a part model...' },
    ...a.addable.map(m => ({ value: m.id, label: m.name })),
  ];
  return (
    <Sheet
      id="assemble-sheet"
      icon="package"
      title="Assemble one prop"
      meta={`${a.parts.length} models · one card on the Board`}
      onClose={a.close}
      width={560}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <section className="flex flex-col gap-2.5">
          <SectionTitle>Prop</SectionTitle>
          <TextField
            label="Prop name"
            mono
            placeholder="sigil_gate"
            value={a.name}
            onChange={e => a.setName(e.target.value)}
            hint={`Exports as ${prop}.glb, the parts as its children.`}
          />
          <Select label="Lands as" options={intoOptions} value={a.into} onChange={a.pickInto} />
        </section>
        <section className="flex flex-col gap-2.5">
          <SectionTitle aside={`${a.parts.length} of ${MAX_ASSEMBLE_PARTS}`}>Parts</SectionTitle>
          <ol className="flex flex-col gap-2">{rows}</ol>
          {a.addable.length && a.parts.length < MAX_ASSEMBLE_PARTS ? (
            <Select
              aria-label="Add a part"
              options={addOptions}
              value=""
              onChange={id => id && a.add(id)}
            />
          ) : null}
          <p className="text-meta text-fg-mute">
            Each part keeps its versions. Redo one and assemble again: the prop picks it up.
          </p>
        </section>
        <section className="flex flex-col gap-2.5">
          <SectionTitle>Placement</SectionTitle>
          <SegmentedControl
            label="Placement"
            tone="clay"
            block
            options={PLACEMENTS}
            value={a.placement}
            onChange={a.setPlacement}
          />
          <div className="rounded-card border border-line bg-raised px-3 py-2.5">
            <p className="font-mono text-meta text-fg">{prop}</p>
            <ul>{tree}</ul>
          </div>
          <p className="text-meta text-fg-mute">
            Each part keeps its pivot, so a door on its left hinge swings on its hinge.
          </p>
        </section>
      </div>
      <footer className="flex flex-col gap-2.5 border-t border-line px-5 py-3">
        <p className={cn('truncate text-meta', a.blocker ? 'text-amber' : 'text-fg-mute')}>
          {a.blocker ?? 'Joins headless in Blender. Spends nothing.'}
        </p>
        <div className="flex items-center justify-end gap-2">
          <Button variant="secondary" onClick={a.close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon="package"
            loading={a.busy}
            disabled={Boolean(a.blocker) || a.busy}
            disabledReason={a.blocker ?? undefined}
            onClick={a.assemble}
          >
            Assemble
          </Button>
        </div>
      </footer>
    </Sheet>
  );
}
