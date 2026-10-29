import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { events } from '@/ipc/generated/bindings';
import { useRefsStore } from '@/state/use-refs-store';
import { clearMocks, openMockRefs } from '@/test/mock-ipc';
import { RefsSheet } from '..';

afterEach(() => {
  clearMocks();
  useRefsStore.setState({ sheet: null, view: null, error: null, busy: false });
});

async function openViews() {
  const mock = await openMockRefs();
  await events.conceptUpdate.listen(e => useRefsStore.getState().onConcept(e.payload));
  render(<RefsSheet />);
  await useRefsStore.getState().open('frostveil', 'guardian-statues', 'views');
  const sheet = await screen.findByRole('dialog', {
    name: 'Make the other views of Guardian statues',
  });
  return { sheet, ...mock };
}

describe('Make the other views', () => {
  it('ticks the empty views and draws only those, with the style board', async () => {
    const { sheet, sent } = await openViews();
    expect(within(sheet).getByRole('checkbox', { name: 'Left' })).toBeChecked();
    expect(within(sheet).getByText('6 Codex images, like the Lab')).toBeInTheDocument();
    await userEvent.click(within(sheet).getByRole('checkbox', { name: 'Back' }));
    await userEvent.click(within(sheet).getByRole('button', { name: 'Draw 2 views' }));
    expect(sent('refs_make_views')[0]).toMatchObject({
      request: { views: ['left', 'right'], count: 2, useStyle: true, notes: null },
    });
  });

  it('sends the notes for every view, trimmed', async () => {
    const { sheet, sent } = await openViews();
    const notes = within(sheet).getByRole('textbox', { name: 'Notes for every view' });
    await userEvent.type(notes, ' The sword in the left hand ');
    expect(within(sheet).getByText(/28 of 400 characters/)).toBeInTheDocument();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Draw 3 views' }));
    expect(sent('refs_make_views')[0]).toMatchObject({
      request: { notes: 'The sword in the left hand' },
    });
  });

  it('shows the candidates per view and puts one in its slot', async () => {
    const { sheet } = await openViews();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Draw 3 views' }));
    const use = await within(sheet).findAllByRole('button', { name: 'Use as Left' });
    expect(use).toHaveLength(2);
    await userEvent.click(use[0]!);
    expect(await within(sheet).findByRole('button', { name: 'In use' })).toBeDisabled();
    const first = useRefsStore.getState().view!.refs.find(r => r.view === 'left')!.file;
    await userEvent.click(within(sheet).getByRole('button', { name: 'Replace Left' }));
    await waitFor(() =>
      expect(useRefsStore.getState().view!.refs.some(r => r.file === first)).toBe(false)
    );
    const lefts = useRefsStore.getState().view!.refs.filter(r => r.view === 'left');
    expect(lefts).toHaveLength(1);
    expect(lefts[0]!.file).not.toBe(first);
  });
});
