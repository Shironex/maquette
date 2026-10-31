import { render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { usePackStore } from '@/state/use-pack-store';
import { clearMocks } from '@/test/mock-ipc';
import { resetPacks, runPack } from '@/test/open-pack';
import PackDone from './PackDone';

afterEach(() => {
  clearMocks();
  resetPacks();
});

describe('a finished pack', () => {
  it('shows what it weighs, what is inside, the manifest and the packs before it', async () => {
    const record = await runPack(3, 'three-props');
    expect(record.state).toBe('done');
    usePackStore.setState({ recent: await commands.packRecent() });
    render(<PackDone record={record} />);
    expect(screen.getByText('3 models passed.')).toBeVisible();
    expect(screen.getByText('all pass')).toBeVisible();
    const inside = screen.getByRole('region', { name: 'What is inside' });
    expect(within(inside).getByText('three-props.zip')).toBeVisible();
    expect(within(inside).getByText('README.txt')).toBeVisible();
    expect(within(inside).getByText('manifest.json')).toBeVisible();
    const manifest = screen.getByRole('region', { name: 'manifest.json' });
    expect(manifest).toHaveTextContent('"pack": "three-props"');
    const recent = screen.getByRole('region', { name: /^Recent packs/ });
    expect(within(recent).getByRole('button', { name: 'Go on from 7' })).toBeVisible();
    expect(within(recent).queryByText('three-props')).toBeNull();
  });
});
