import { fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
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
    expect(screen.getByText('Blender OBJ import, 3 of 3')).toBeVisible();
    expect(screen.queryByText(/GLB import/)).toBeNull();
    const recent = screen.getByRole('region', { name: /^Earlier packs/ });
    expect(within(recent).getByRole('button', { name: 'Go on from 7' })).toBeVisible();
    expect(within(recent).queryByText('three-props')).toBeNull();
  });

  it('shows the pictures it was made from when the contact sheet will not load', async () => {
    const record = await runPack(3, 'three-props');
    registerMockFile(`${record.dir}/contact-sheet.png`, 'mock://sheet');
    render(<PackDone record={record} />);
    const sheet = screen.getByRole('img', { name: 'Every model in three-props' });
    expect(sheet.tagName).toBe('IMG');
    fireEvent.error(sheet);
    const list = screen.getByRole('list', { name: 'Every model in three-props' });
    expect(within(list).getAllByRole('listitem')).toHaveLength(3);
  });
});
