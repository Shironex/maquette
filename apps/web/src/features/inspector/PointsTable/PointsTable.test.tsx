import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePathsStore } from '@/state/use-paths-store';
import { clearMocks, openPaths } from '@/test/open-paths';
import PointsTable from './PointsTable';
import { nextPoint } from './PointsTable.hooks';

afterEach(clearMocks);

const path = (id: string) => usePathsStore.getState().draft.paths.find(p => p.id === id)!;

describe('the points table', () => {
  it('lists each point with how far along it is, and picks one', async () => {
    const s = await openPaths();
    render(<PointsTable path={path('root-glow')} models={s.view!.models} />);
    expect(screen.getAllByRole('row')).toHaveLength(4);
    expect(screen.getByText('1.87 m')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Pick point 2' }));
    expect(usePathsStore.getState().selection).toEqual({ path: 'root-glow', point: 1 });
  });

  it('moves a point by its fields and adds one at the end', async () => {
    const s = await openPaths();
    const { rerender } = render(<PointsTable path={path('spare-run')} models={s.view!.models} />);
    const y = screen.getByRole('textbox', { name: 'Point 2 Y' });
    await userEvent.clear(y);
    await userEvent.type(y, '0.4{Enter}');
    expect(path('spare-run').points[1]).toEqual([2.5, 0.4, -2.6]);
    rerender(<PointsTable path={path('spare-run')} models={s.view!.models} />);
    await userEvent.click(screen.getByRole('button', { name: 'Add point at the end' }));
    expect(path('spare-run').points).toHaveLength(3);
    expect(usePathsStore.getState().selection).toEqual({ path: 'spare-run', point: 2 });
  });

  it('keeps two points: their remove buttons are off', async () => {
    const s = await openPaths();
    render(<PointsTable path={path('spare-run')} models={s.view!.models} />);
    expect(screen.getByRole('button', { name: 'Remove point 1' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('puts the next point half a metre on along the last run', () => {
    const next = nextPoint({
      ...path('spare-run'),
      points: [
        [0, 0, 0],
        [1, 0, 0],
      ],
    });
    expect(next).toEqual([1.5, 0, 0]);
  });
});
