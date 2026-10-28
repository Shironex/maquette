import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
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
});
