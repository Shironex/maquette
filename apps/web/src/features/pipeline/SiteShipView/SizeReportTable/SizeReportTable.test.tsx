import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { SizeReport, SizeRow } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { SizeReportTable } from '.';
import { keptShare } from './SizeReportTable.hooks';

afterEach(() => useAppStore.setState({ notice: null }));

const desk: SizeRow = {
  asset: 'desk',
  name: 'Desk',
  version: 2,
  sourceFile: 'models/desk_v2.glb',
  sourceBytes: 4_000_000,
  sourceTriangles: 40_000,
  shippedFile: 'web/dist/desk.glb',
  shippedBytes: 1_000_000,
  shippedTriangles: 20_000,
  exportBytes: 3_000_000,
  grew: null,
};

const report: SizeReport = {
  project: 'room',
  textures: 'webp',
  rows: [desk],
  totals: {
    models: 1,
    sourceBytes: 4_000_000,
    shippedBytes: 1_000_000,
    sourceTriangles: 40_000,
    shippedTriangles: 20_000,
  },
  csv: 'asset,name\ndesk,Desk',
  markdown: '| Model |\n| Desk |',
};

describe('the size report', () => {
  it('shows each model against what ships and copies the CSV', async () => {
    const user = userEvent.setup();
    render(<SizeReportTable report={report} />);
    expect(screen.getByText(/WebP textures/)).toBeInTheDocument();
    expect(screen.getAllByText('25%')).toHaveLength(2);
    await user.click(screen.getByRole('button', { name: 'Copy CSV' }));
    expect(await navigator.clipboard.readText()).toBe('asset,name\ndesk,Desk');
    expect(useAppStore.getState().notice?.title).toBe('The size report as CSV copied');
  });

  it('works out the kept share only when both sides are measured', () => {
    expect(keptShare(null, 10)).toEqual({ text: '·', grew: false });
    expect(keptShare(200, 50)).toEqual({ text: '25%', grew: false });
  });

  it('flags a model that grew instead of keeping more than all of it', () => {
    expect(keptShare(22_784, 58_632)).toEqual({ text: '+157%', grew: true });
    const why =
      'Larger than the source: Bake light added 2 lightmaps and their UV set. Compress kept 21% of the 271 KB export.';
    const row = {
      ...desk,
      asset: 'zabuton',
      name: 'Zabuton',
      sourceBytes: 22_784,
      shippedBytes: 58_632,
      exportBytes: 277_728,
      grew: why,
    };
    render(
      <SizeReportTable
        report={{
          ...report,
          rows: [row],
          totals: { ...report.totals, sourceBytes: 22_784, shippedBytes: 58_632 },
        }}
      />
    );
    expect(screen.queryByText(/257%/)).not.toBeInTheDocument();
    expect(screen.getAllByText('+157%')).toHaveLength(2);
    expect(screen.getByText(why)).toBeInTheDocument();
  });
});
