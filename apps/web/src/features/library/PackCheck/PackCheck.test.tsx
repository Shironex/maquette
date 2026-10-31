import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePackStore } from '@/state/use-pack-store';
import { clearMocks } from '@/test/mock-ipc';
import { follow, resetPacks, runPack } from '@/test/open-pack';
import PackCheck from './PackCheck';

afterEach(() => {
  clearMocks();
  resetPacks();
});

describe('the check of a pack', () => {
  it('shows a card per model and waits for the models that need you', async () => {
    const record = await runPack(8);
    expect(record.state).toBe('waiting');
    render(<PackCheck record={record} />);
    const models = screen.getByRole('region', { name: 'Models in the pack' });
    expect(within(models).getAllByRole('article')).toHaveLength(8);
    const needs = screen.getByRole('complementary', { name: 'Needs you' });
    expect(within(needs).getAllByRole('article')).toHaveLength(2);
    expect(within(needs).getByRole('button', { name: /^Raise to / })).toBeVisible();
    expect(within(needs).getByRole('button', { name: 'Export again' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Zip 6 that pass' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Zip all 7' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Stop' })).toHaveAttribute('aria-disabled', 'true');
    expect(within(needs).getByText(/^village-props, 8 models/)).toBeVisible();
  });

  it('filters the cards by what the check found', async () => {
    const record = await runPack(8);
    render(<PackCheck record={record} />);
    await userEvent.click(screen.getByRole('tab', { name: /^Failed/ }));
    const models = screen.getByRole('region', { name: 'Models in the pack' });
    const failed = within(models).getAllByRole('article');
    expect(failed).toHaveLength(1);
    expect(within(failed[0]!).getByText(/^Texture missing/)).toBeVisible();
  });

  it('raises a budget, exports a failed model again and zips once every model passes', async () => {
    const record = await runPack(8);
    const { rerender } = render(<PackCheck record={record} />);
    const needs = screen.getByRole('complementary', { name: 'Needs you' });
    await userEvent.click(within(needs).getByRole('button', { name: /^Raise to / }));
    let now = await follow(record.id, ['waiting']);
    rerender(<PackCheck record={now} />);
    await userEvent.click(within(needs).getByRole('button', { name: 'Export again' }));
    now = await follow(record.id, ['done']);
    expect(now.items.every(i => i.state === 'pass')).toBe(true);
    expect(now.zip).toMatch(/village-props\.zip$/);
    await waitFor(() => expect(usePackStore.getState().record?.state).toBe('done'));
  });
});
