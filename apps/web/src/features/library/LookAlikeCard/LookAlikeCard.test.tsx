import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type LookAlikePair } from '@/ipc/client';
import { useLookAlikesStore } from '@/state/use-look-alikes-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LookAlikeCard from './LookAlikeCard';

afterEach(() => {
  clearMocks();
  useLookAlikesStore.setState({ report: null, busy: null });
});

async function pairOf(kind: LookAlikePair['kind']) {
  await openMockLibrary('scale');
  const report = await commands.libraryLookAlikes();
  useLookAlikesStore.setState({ report });
  const items = (await commands.libraryGet()).items;
  const pair = report.pairs.find(p => p.kind === kind)!;
  const byId = (id: string) => items.find(i => i.entry.id === id)!;
  return { pair, a: byId(pair.a), b: byId(pair.b) };
}

describe('a look-alike pair', () => {
  it('folded, waits on one line for Review', async () => {
    const { pair, a, b } = await pairOf('same-look');
    const onOpen = vi.fn();
    render(
      <ul>
        <LookAlikeCard pair={pair} a={a} b={b} open={false} onOpen={onOpen} />
      </ul>
    );
    await userEvent.click(screen.getByRole('button', { name: 'Review' }));
    expect(onOpen).toHaveBeenCalled();
  });

  it('merges a same look pair as versions of the older model', async () => {
    const { pair, a, b } = await pairOf('same-look');
    render(
      <ul>
        <LookAlikeCard pair={pair} a={a} b={b} open onOpen={() => {}} />
      </ul>
    );
    const card = screen.getByRole('listitem', { name: `${a.entry.name} and ${b.entry.name}` });
    expect(within(card).getByText('Outline')).toBeVisible();
    await userEvent.click(within(card).getByRole('button', { name: 'Merge as versions' }));
    await waitFor(async () => {
      const ids = (await commands.libraryGet()).items.map(i => i.entry.id);
      expect(ids).toContain(a.entry.id);
      expect(ids).not.toContain(b.entry.id);
    });
  });

  it('wants two different names before Rename both on a same name pair', async () => {
    const { pair, a, b } = await pairOf('same-name');
    render(
      <ul>
        <LookAlikeCard pair={pair} a={a} b={b} open onOpen={() => {}} />
      </ul>
    );
    const rename = screen.getByRole('button', { name: 'Rename both' });
    const second = screen.getByRole('textbox', { name: 'Second' });
    await userEvent.clear(second);
    await userEvent.type(second, a.entry.name);
    expect(rename).toHaveAttribute('aria-disabled', 'true');
    await userEvent.clear(second);
    await userEvent.type(second, `${a.entry.name} by the gate`);
    expect(rename).not.toHaveAttribute('aria-disabled');
    expect(rename).toBeEnabled();
  });
});
