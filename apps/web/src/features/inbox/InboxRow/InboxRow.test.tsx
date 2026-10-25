import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useInboxStore } from '@/state/use-inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import InboxRow from './InboxRow';

beforeEach(async () => {
  await openMockWorkspace();
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
});

describe('InboxRow', () => {
  it('shows the model with its numbers and status, and picks it on click', async () => {
    const item = useInboxStore.getState().snapshot!.items.find(i => i.id === 'character.glb')!;
    const onSelect = vi.fn();
    render(<InboxRow item={item} selected={false} onSelect={onSelect} />);
    const row = screen.getByRole('button', { name: /Character/ });
    expect(row).toHaveAttribute('aria-pressed', 'false');
    expect(row).toHaveTextContent('94.6k tris · 3 textures');
    expect(row).toHaveTextContent('needs review');
    await userEvent.click(row);
    expect(onSelect).toHaveBeenCalledOnce();
  });

  it('ticks the model for a batch redo and opens its menu', async () => {
    const item = useInboxStore.getState().snapshot!.items.find(i => i.id === 'character.glb')!;
    const onCheck = vi.fn();
    const onMenu = vi.fn();
    const { rerender } = render(
      <InboxRow
        item={item}
        selected={false}
        onSelect={() => {}}
        onCheck={onCheck}
        onMenu={onMenu}
        onCloseMenu={() => {}}
      />
    );
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Character' }));
    expect(onCheck).toHaveBeenCalledWith(true);
    await userEvent.click(screen.getByRole('button', { name: 'More for Character' }));
    expect(onMenu).toHaveBeenCalledOnce();
    rerender(
      <InboxRow
        item={item}
        selected={false}
        onSelect={() => {}}
        checked
        onCheck={onCheck}
        menuOpen
        onMenu={onMenu}
        onCloseMenu={() => {}}
      />
    );
    expect(screen.getByRole('checkbox', { name: 'Select Character' })).toHaveAttribute(
      'aria-checked',
      'true'
    );
    expect(screen.getByRole('menu', { name: 'Character actions' })).toBeInTheDocument();
  });
});
