import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { SizeReport } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { SizeReportTable } from '.';
import { keptShare } from './SizeReportTable.hooks';

afterEach(() => useAppStore.setState({ notice: null }));

const report: SizeReport = {
  project: 'room',
  textures: 'webp',
  rows: [
    {
      asset: 'desk',
      name: 'Desk',
      version: 2,
      sourceFile: 'models/desk_v2.glb',
      sourceBytes: 4_000_000,
      sourceTriangles: 40_000,
      shippedFile: 'web/dist/desk.glb',
      shippedBytes: 1_000_000,
      shippedTriangles: 20_000,
    },
  ],
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
    expect(keptShare(null, 10)).toBe('·');
    expect(keptShare(200, 50)).toBe('25%');
  });
});
