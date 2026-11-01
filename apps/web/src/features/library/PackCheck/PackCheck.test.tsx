import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { PackRecord } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { newItem } from '@/ipc/mock/pack-fixtures';
import { usePackStore } from '@/state/use-pack-store';
import { clearMocks } from '@/test/mock-ipc';
import { follow, resetPacks, runPack } from '@/test/open-pack';
import PackCheck from './PackCheck';
import PackCheckCard from './PackCheckCard';

afterEach(() => {
  clearMocks();
  resetPacks();
});

const failedKey = (record: PackRecord) => record.items.find(i => i.state === 'fail')!.model.key;

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
    expect(screen.getByRole('button', { name: 'Zip 7 of 8' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Stop' })).toHaveAttribute('aria-disabled', 'true');
    expect(within(needs).getByText(/^village-props, 8 models/)).toBeVisible();
  });

  it('answers every model with the same warning in one go', async () => {
    const record = await runPack(8);
    const thin = (key: string, n: number) => ({
      ...record.items[n]!,
      model: { ...record.items[n]!.model, key, name: key },
      state: 'warn' as const,
      budget: 3000,
      suggest: 6000,
      note: 'Thin parts lost at 3k tris; 6k keeps more of them.',
    });
    const items = [thin('a', 0), thin('b', 1), thin('c', 2)];
    const shown = { ...record, items };
    usePackStore.setState({ record: shown });
    const said: unknown[] = [];
    const decideAll = usePackStore.getState().decideAll;
    usePackStore.setState({
      decideAll: async d => {
        said.push(d);
      },
    });
    render(<PackCheck record={shown} />);
    const group = screen.getByRole('group', { name: 'Thin parts lost, 3 models' });
    await userEvent.click(within(group).getByRole('button', { name: 'Raise all' }));
    await userEvent.click(within(group).getByRole('button', { name: 'Keep all' }));
    expect(said[0]).toEqual(
      ['a', 'b', 'c'].map(key => ({ choice: 'raise', key, triangles: 6000 }))
    );
    expect(said[1]).toEqual(['a', 'b', 'c'].map(key => ({ choice: 'keep', key })));
    usePackStore.setState({ decideAll });
  });

  it('keeps every warning through the real answers and finishes the pack', async () => {
    const record = await runPack(8);
    await usePackStore.getState().decide({ choice: 'again', key: failedKey(record) });
    const waiting = await follow(record.id, ['waiting', 'done']);
    const warned = waiting.items.filter(i => i.state === 'warn');
    await usePackStore
      .getState()
      .decideAll(warned.map(i => ({ choice: 'keep', key: i.model.key })));
    const now = await follow(record.id, ['waiting', 'done', 'zipping']);
    expect(now.items.filter(i => i.state === 'warn')).toHaveLength(0);
    expect(now.items.filter(i => i.state === 'kept')).toHaveLength(warned.length);
  });

  it('offers no group for a warning only one model has', async () => {
    const record = await runPack(8);
    render(<PackCheck record={record} />);
    expect(screen.queryByRole('button', { name: 'Keep all' })).toBeNull();
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

  it('counts the model running apart from the models waiting', async () => {
    const record = await runPack(8);
    const items = record.items.map((item, n) => ({
      ...item,
      state: n === 0 ? ('pass' as const) : n === 1 ? ('running' as const) : ('queued' as const),
    }));
    render(<PackCheck record={{ ...record, state: 'running', items }} />);
    expect(screen.getByRole('tab', { name: /^Running/ })).toHaveTextContent('1');
    expect(screen.getByRole('tab', { name: /^Waiting/ })).toHaveTextContent('6');
    await userEvent.click(screen.getByRole('tab', { name: /^Waiting/ }));
    const models = screen.getByRole('region', { name: 'Models in the pack' });
    expect(within(models).getAllByRole('article')).toHaveLength(6);
  });

  it('says it is stopping until the model on now is done, and goes on from the next', async () => {
    const record = await runPack(8);
    const items = record.items.map((item, n) => ({
      ...item,
      state: n < 7 ? ('pass' as const) : ('running' as const),
    }));
    usePackStore.setState({ stopping: record.id });
    const { rerender } = render(<PackCheck record={{ ...record, state: 'running', items }} />);
    expect(screen.getByText(/stopping after this model/)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Stopping' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    usePackStore.setState({ stopping: null });
    const stopped = items.map((item, n) =>
      n === 7 ? { ...item, state: 'queued' as const } : item
    );
    const message = 'Stopped with 7 of 8 done, model 8 is next';
    rerender(<PackCheck record={{ ...record, state: 'stopped', items: stopped, message }} />);
    expect(screen.getByText(message)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Go on from 8' })).toBeVisible();
  });
});

describe('an answer on a stopped pack', () => {
  it('says the pack stays stopped, and a running or waiting one says nothing of it', async () => {
    const record = await runPack(8);
    const stopped = { ...record, state: 'stopped' as const };
    const { rerender } = render(<PackCheck record={stopped} />);
    let needs = screen.getByRole('complementary', { name: 'Needs you' });
    await userEvent.hover(within(needs).getByRole('button', { name: /^Raise to / }));
    expect((await screen.findByRole('tooltip')).textContent).toContain(
      'Decimates only this model again'
    );
    await userEvent.unhover(within(needs).getByRole('button', { name: /^Raise to / }));
    await userEvent.hover(within(needs).getByRole('button', { name: /^Keep / }));
    expect((await screen.findByRole('tooltip')).textContent).toContain('The pack stays stopped');
    rerender(<PackCheck record={record} />);
    needs = screen.getByRole('complementary', { name: 'Needs you' });
    await userEvent.hover(within(needs).getByRole('button', { name: /^Keep / }));
    expect((await screen.findByRole('tooltip')).textContent).not.toContain('stays stopped');
  });
});

describe('the picture on a card', () => {
  const model = (thumbnail: string | null) => ({
    key: 'crate',
    name: 'Crate',
    kind: 'prop' as const,
    entry: 'crate',
    project: null,
    asset: null,
    version: 1,
    file: '/ws/crate.glb',
    stats: null,
    rigged: false,
    heightM: null,
    mixamo: false,
    source: null,
    licence: '',
    thumbnail,
    refs: [],
    prep: null,
  });
  const src = (card: HTMLElement) => card.querySelector('img')?.getAttribute('src');

  it('gives way to the library picture, then the kind icon, never a broken image', () => {
    registerMockFile('/ws/exports/.packs/1/crate.png', 'mock://render');
    registerMockFile('/ws/library/crate/thumb.png', 'mock://thumb');
    const item = {
      ...newItem(model('/ws/library/crate/thumb.png'), 3000),
      state: 'pass' as const,
      picture: '/ws/exports/.packs/1/crate.png',
    };
    const { container } = render(<PackCheckCard item={item} format="obj" />);
    expect(src(container)).toBe('mock://render');
    fireEvent.error(container.querySelector('img')!);
    expect(src(container)).toBe('mock://thumb');
    fireEvent.error(container.querySelector('img')!);
    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('tries again when a new run gives it a new picture', () => {
    registerMockFile('/ws/a.png', 'mock://a');
    registerMockFile('/ws/b.png', 'mock://b');
    const item = { ...newItem(model(null), 3000), state: 'pass' as const, picture: '/ws/a.png' };
    const { container, rerender } = render(<PackCheckCard item={item} format="obj" />);
    fireEvent.error(container.querySelector('img')!);
    expect(container.querySelector('img')).toBeNull();
    rerender(<PackCheckCard item={{ ...item, picture: '/ws/b.png' }} format="obj" />);
    expect(src(container)).toBe('mock://b');
  });

  it('says on the numbers which budget the model ran under', () => {
    const item = {
      ...newItem({ ...model(null), kind: 'character' }, null),
      state: 'pass' as const,
      triangles: 5999,
    };
    render(<PackCheckCard item={item} format="obj" />);
    expect(screen.getByTitle('5,999 tris · Characters, no budget')).toBeVisible();
  });
});
