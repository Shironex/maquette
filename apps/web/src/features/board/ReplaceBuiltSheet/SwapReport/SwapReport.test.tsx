import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { ReplaceReport, SwapSide } from '@/ipc/client';
import SwapReport from './SwapReport';

const side = (over: Partial<SwapSide> = {}): SwapSide => ({
  version: 2,
  file: 'models/crystal_lantern_v2.glb',
  shipsAs: 'crystal_lantern_v2.glb',
  triangles: 12_900,
  bytes: 400_000,
  min: [-0.68, 0, -0.57],
  max: [0.68, 1.8, 0.57],
  scale: 1.38,
  ...over,
});

const report = (over: Partial<ReplaceReport> = {}): ReplaceReport => ({
  project: 'frostveil',
  asset: 'crystal-lantern',
  name: 'Crystal lantern',
  built: 'lantern-rebuilt',
  builtName: 'Lantern rebuilt',
  script: 'built/crystal_lantern.py',
  old: side(),
  new: side({
    version: 3,
    file: 'models/crystal_lantern_v3.glb',
    shipsAs: 'crystal_lantern_v3.glb',
    triangles: 1_840,
    bytes: 96_000,
    scale: 1,
  }),
  bakes: true,
  parts: ['lantern', 'cage', 'crystal'],
  settings: [
    { what: 'Real size, 1.8 m high', action: 'carried', why: 'It is a measure.' },
    { what: 'Decimate to 60k', action: 'reset', why: 'It repaired the old model.' },
    { what: 'Skips the bake', action: 'reset', why: 'A built model bakes.' },
    { what: 'Layout placement: position 3.6, 0, 1.2', action: 'kept', why: 'Keyed by the id.' },
  ],
  unresolved: [],
  warnings: [],
  note: 'Nothing is aligned.',
  trash: null,
  ...over,
});

describe('SwapReport', () => {
  it('puts the old and the built model side by side with their numbers', () => {
    render(<SwapReport report={report()} />);
    const table = screen.getByRole('table', {
      name: 'Crystal lantern: the old model and the built model that takes its place',
    });
    const cells = (label: string) =>
      within(within(table).getByRole('row', { name: new RegExp(`^${label}`) }))
        .getAllByRole('cell')
        .map(c => c.textContent);
    expect(cells('File')).toEqual([
      'models/crystal_lantern_v2.glb',
      'models/crystal_lantern_v3.glb',
    ]);
    expect(cells('Ships as')).toEqual(['crystal_lantern_v2.glb', 'crystal_lantern_v3.glb']);
    expect(cells('Model')).toEqual(['12.9k triangles · 400 KB', '1.8k triangles · 96 KB']);
    expect(cells('Stands')).toEqual(['1.4 × 1.8 × 1.1 m', '1.4 × 1.8 × 1.1 m']);
    expect(cells('Real size')).toEqual(['scaled by 1.38', 'not scaled']);
    expect(cells('Bounds, m')).toEqual([
      '-0.68, 0, -0.57\nto 0.68, 1.8, 0.57',
      '-0.68, 0, -0.57\nto 0.68, 1.8, 0.57',
    ]);
    expect(within(table).getByRole('columnheader', { name: 'Now, v2' })).toBeInTheDocument();
    expect(within(table).getByRole('columnheader', { name: 'Built, v3' })).toBeInTheDocument();
    expect(screen.getByText(/bakes its light like every other built piece/)).toBeInTheDocument();
    expect(screen.getByText(/Its parts: lantern, cage, crystal\./)).toBeInTheDocument();
    expect(screen.getByText('Nothing is aligned.')).toBeInTheDocument();
  });

  it('groups every setting by what happens to it, with the reason', () => {
    render(<SwapReport report={report()} />);
    const off = screen.getByRole('region', { name: 'Taken off' });
    expect(
      within(off)
        .getAllByRole('listitem')
        .map(i => i.textContent)
    ).toEqual(['Decimate to 60kIt repaired the old model.', 'Skips the bakeA built model bakes.']);
    expect(
      within(screen.getByRole('region', { name: 'Carries over' })).getByRole('listitem')
    ).toHaveTextContent('Real size, 1.8 m highIt is a measure.');
    expect(screen.getByRole('region', { name: 'Stays as it is' })).toHaveTextContent(
      'Layout placement'
    );
    expect(screen.queryByRole('region', { name: 'Comes with the built asset' })).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('lists what to check, and says so when the model skips the bake', () => {
    render(
      <SwapReport
        report={report({
          bakes: false,
          parts: [],
          warnings: ['The built model stands 9 cm where the old model stood 30 cm.'],
          unresolved: ['The state Open moves the part Crate, which the built model does not have.'],
        })}
      />
    );
    const check = screen.getByRole('status');
    expect(check).toHaveTextContent('2 to check');
    expect(
      within(check)
        .getAllByRole('listitem')
        .map(i => i.textContent)
    ).toEqual([
      'The built model stands 9 cm where the old model stood 30 cm.',
      'The state Open moves the part Crate, which the built model does not have.',
    ]);
    expect(screen.getByText(/leaves it out of the bake/)).toBeInTheDocument();
    expect(screen.getByText(/Its parts: none read\./)).toBeInTheDocument();
  });
});
