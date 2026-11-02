import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useRefsStore } from '@/state/use-refs-store';
import { clearMocks, openMockRefs } from '@/test/mock-ipc';
import { RefsBody } from '.';

afterEach(() => {
  clearMocks();
  useRefsStore.setState({ sheet: null, view: null, error: null, busy: false });
});

async function body() {
  await openMockRefs();
  await useRefsStore.getState().open('frostveil', 'guardian-statues');
  const onSend = vi.fn();
  const view = () => useRefsStore.getState().view!;
  const { rerender } = render(<RefsBody view={view()} warnings={[]} onSend={onSend} />);
  return { again: () => rerender(<RefsBody view={view()} warnings={[]} onSend={onSend} />) };
}

describe('the refs page', () => {
  it('says how a send uses the views', async () => {
    await body();
    expect(screen.getByText('How a send uses the views')).toBeInTheDocument();
    expect(screen.getByText(/front.png, left.png, back.png, right.png/)).toBeInTheDocument();
    expect(screen.getByText('1 of 4 views · 3 refs')).toBeInTheDocument();
  });

  it('opens Make the other views from the button or an empty slot', async () => {
    await body();
    await userEvent.click(screen.getByRole('button', { name: 'Make the other views' }));
    expect(useRefsStore.getState().sheet?.mode).toBe('views');
    useRefsStore.getState().show('refs');
    await userEvent.click(screen.getAllByRole('button', { name: /Empty, draw it/ })[0]!);
    expect(useRefsStore.getState().sheet?.mode).toBe('views');
  });

  it('adds an image from refs/ as an other ref', async () => {
    const { again } = await body();
    await userEvent.click(screen.getByRole('button', { name: 'Add refs' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'veil_gate_front.png' }));
    again();
    expect(
      await screen.findByRole('button', { name: 'View of veil_gate_front.png: Other' })
    ).toBeInTheDocument();
  });

  it('takes a ref from disk when refs/ has nothing to offer, as its front', async () => {
    const { again } = await body();
    const view = useRefsStore.getState().view!;
    useRefsStore.setState({ view: { ...view, refs: [], images: [], tagged: false } });
    again();
    await userEvent.click(screen.getByRole('button', { name: 'Add refs' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Choose from disk...' }));
    await waitFor(() => {
      const refs = useRefsStore.getState().view!.refs;
      expect(refs).toContainEqual(
        expect.objectContaining({ file: 'refs/from_disk.png', view: 'front' })
      );
    });
  });

  it('keeps the tags already there when it takes a ref from disk', async () => {
    const { again } = await body();
    const front = useRefsStore.getState().view!.refs.find(r => r.view === 'front')!;
    again();
    await userEvent.click(screen.getByRole('button', { name: 'Add refs' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Choose from disk...' }));
    await waitFor(() =>
      expect(useRefsStore.getState().view!.refs).toContainEqual(
        expect.objectContaining({ file: 'refs/from_disk.png', view: 'other' })
      )
    );
    expect(useRefsStore.getState().view!.refs.find(r => r.view === 'front')?.file).toBe(front.file);
  });

  it('says what went wrong when a file from disk is no image', async () => {
    await body();
    vi.spyOn(commands, 'dialogPickFiles').mockResolvedValue(['/art/notes.txt'] as never);
    await userEvent.click(screen.getByRole('button', { name: 'Add refs' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Choose from disk...' }));
    await waitFor(() => expect(useAppStore.getState().notice?.detail).toContain('notes.txt'));
    vi.restoreAllMocks();
  });
});
