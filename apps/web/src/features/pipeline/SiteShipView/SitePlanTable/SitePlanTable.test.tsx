import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { SitePlanFile } from '@/ipc/client';
import { SitePlanTable } from '.';

const file = (patch: Partial<SitePlanFile>): SitePlanFile => ({
  file: 'desk.glb',
  asset: 'desk',
  assetName: 'Desk',
  change: 'replaced',
  choice: 'auto',
  ships: true,
  held: null,
  bytes: 410_000,
  beforeBytes: 1_200_000,
  triangles: 9_000,
  beforeTriangles: 14_000,
  ...patch,
});

describe('the site plan table', () => {
  it('shows before and after, and why a file is held', () => {
    render(
      <SitePlanTable
        files={[
          file({}),
          file({
            file: 'lamp.glb',
            asset: null,
            assetName: null,
            ships: false,
            held: 'set to never ship',
            beforeBytes: null,
            beforeTriangles: null,
          }),
        ]}
        left={[{ file: 'old.glb', bytes: 90_000, removed: true }]}
        fileOf={() => null}
        editable
        onChoice={vi.fn()}
        onFile={vi.fn()}
      />
    );
    expect(screen.getByText('1.2 MB → 410 KB')).toBeInTheDocument();
    expect(screen.getByText('14k → 9k')).toBeInTheDocument();
    expect(screen.getByText('Held: set to never ship')).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Ship lamp.glb' })).toBeDisabled();
    expect(screen.getByText('Maquette shipped it before, so Ship removes it')).toBeInTheDocument();
  });
});
