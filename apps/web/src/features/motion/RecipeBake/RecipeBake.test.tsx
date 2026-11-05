import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { record } from '@/test/recipe-bake-fixture';
import RecipeBake from './RecipeBake';

describe('a recipe bake panel', () => {
  it('shows a running bake with its progress and Cancel', async () => {
    const onCancel = vi.fn();
    const running = record({
      state: 'running',
      result: null,
      progress: { label: 'Solving frame 12 of 31', done: 3, total: 8 },
    });
    render(<RecipeBake record={running} busy={false} error={null} onCancel={onCancel} />);
    const bar = screen.getByRole('progressbar', { name: 'Baking Reach out' });
    expect(bar).toHaveAttribute('aria-valuenow', '38');
    expect(screen.getByText('Solving frame 12 of 31')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onCancel).toHaveBeenCalled();
  });

  it('shows a finished bake with its numbers against their limits and Review', async () => {
    const onReview = vi.fn();
    render(
      <RecipeBake
        record={record()}
        busy={false}
        error={null}
        onCancel={() => {}}
        onReview={onReview}
      />
    );
    const lines = screen.getByRole('list', { name: 'Bake result' });
    expect(within(lines).getByText('0.4 cm')).toBeVisible();
    expect(within(lines).getByText('3.2 cm')).toBeVisible();
    expect(within(lines).getAllByRole('img', { name: 'Worth a look' })).toHaveLength(2);
    expect(within(lines).getByText('Foot slide')).toBeVisible();
    expect(screen.getByText('2 of 6 numbers warn')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Review' }));
    expect(onReview).toHaveBeenCalled();
    expect(screen.getByText('Mixamo')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Cancel' })).toBeNull();
  });

  it('shows an added bake as its version, with what became of a same-named clip', () => {
    const added = record({
      state: 'added',
      addedVersion: 3,
      result: { ...record().result!, clipNote: 'Elsheria already has a clip named reach_out.' },
    });
    render(<RecipeBake record={added} busy={false} error={null} onCancel={() => {}} />);
    expect(screen.getByText('Added as v3 of Elsheria.')).toBeVisible();
    expect(screen.getByText('Elsheria already has a clip named reach_out.')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Review' })).toBeNull();
  });

  it('marks a bake out of date with the reason', () => {
    render(
      <RecipeBake
        record={record()}
        busy={false}
        error={null}
        stale="The recipe changed since this bake. Bake it again."
        onCancel={() => {}}
      />
    );
    expect(screen.getByText('Out of date')).toBeVisible();
    expect(screen.getByText('The recipe changed since this bake. Bake it again.')).toBeVisible();
  });

  it('says why a bake failed or did not start, and nothing before a first bake', () => {
    const failed = record({ state: 'failed', result: null, error: 'Blender stopped.' });
    const { rerender } = render(
      <RecipeBake record={failed} busy={false} error={null} onCancel={() => {}} />
    );
    expect(screen.getByText('Blender stopped.')).toBeVisible();
    rerender(<RecipeBake record={null} busy={false} error="No rig." onCancel={() => {}} />);
    expect(screen.getByText('The bake did not start')).toBeVisible();
    rerender(<RecipeBake record={null} busy={false} error={null} onCancel={() => {}} />);
    expect(screen.queryByText(/bake/i)).toBeNull();
  });
});
