import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { UndoReport } from '@/ipc/client';
import UndoSummary from './UndoSummary';

const report = (over: Partial<UndoReport> = {}): UndoReport => ({
  project: 'frostveil',
  asset: 'crystal-lantern',
  name: 'Crystal lantern',
  script: 'built/crystal_lantern.py',
  back: {
    version: 2,
    file: 'models/crystal_lantern_v2.glb',
    triangles: 12_900,
    bytes: 400_000,
  },
  takenOff: [
    { version: 3, file: 'models/crystal_lantern_v3.glb', triangles: 1_840, bytes: 96_000 },
    { version: 4, file: 'models/crystal_lantern_v4.glb', triangles: null, bytes: null },
  ],
  notBuilt: [],
  setAside: null,
  restores: 'Lantern rebuilt',
  settings: ['Real size, 1.8 m high', 'Decimate to 60k', 'Skips the bake'],
  warnings: [],
  ...over,
});

describe('UndoSummary', () => {
  it('says which model is current again and which versions leave', () => {
    render(<UndoSummary report={report()} />);
    expect(screen.getByRole('region', { name: 'Current again' })).toHaveTextContent(
      'v2 models/crystal_lantern_v2.glb12.9k triangles · 400 KB'
    );
    const off = screen.getByRole('region', { name: 'Leaves the asset' });
    expect(
      within(off)
        .getAllByRole('listitem')
        .map(i => i.textContent)
    ).toEqual([
      'v3 models/crystal_lantern_v3.glb1.8k triangles · 96 KB',
      'v4 models/crystal_lantern_v4.glb',
    ]);
    expect(off).toHaveTextContent('Their files move to the workspace trash. Nothing is deleted');
    expect(off).toHaveTextContent('built/crystal_lantern.py stops building this asset');
  });

  it('lists what comes back with the model, and the built asset card', () => {
    render(<UndoSummary report={report()} />);
    const back = screen.getByRole('region', { name: 'Comes back' });
    expect(
      within(back)
        .getAllByRole('listitem')
        .map(i => i.textContent)
    ).toEqual(['Real size, 1.8 m high', 'Decimate to 60k', 'Skips the bake']);
    expect(
      screen.getByText(/Lantern rebuilt comes back on the Board as a built asset of its own/)
    ).toBeInTheDocument();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('says so when the card cannot come back, and skips the empty parts', () => {
    render(
      <UndoSummary
        report={report({
          restores: null,
          settings: [],
          warnings: ['Lantern rebuilt is not in the asset trash any more.'],
        })}
      />
    );
    expect(screen.queryByRole('region', { name: 'Comes back' })).toBeNull();
    expect(screen.queryByText(/comes back on the Board/)).toBeNull();
    expect(screen.getByRole('status')).toHaveTextContent(
      '1 to knowLantern rebuilt is not in the asset trash any more.'
    );
  });
});
