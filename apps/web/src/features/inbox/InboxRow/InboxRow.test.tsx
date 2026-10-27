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
    const item = useInboxStore.getState().snapshot!.items.find(i => i.id === 'elsheria.glb')!;
    const onSelect = vi.fn();
    render(<InboxRow item={item} selected={false} onSelect={onSelect} />);
    const row = screen.getByRole('button', { name: /Elsheria/ });
    expect(row).toHaveAttribute('aria-pressed', 'false');
    expect(row).toHaveTextContent('115.7k tris · 1 texture');
    expect(row).toHaveTextContent('needs review');
    await userEvent.click(row);
    expect(onSelect).toHaveBeenCalledOnce();
  });

  it('ticks the model for a batch redo and opens its menu', async () => {
    const item = useInboxStore.getState().snapshot!.items.find(i => i.id === 'elsheria.glb')!;
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
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Elsheria' }));
    expect(onCheck).toHaveBeenCalledWith(true);
    await userEvent.click(screen.getByRole('button', { name: 'More for Elsheria' }));
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
    expect(screen.getByRole('checkbox', { name: 'Select Elsheria' })).toHaveAttribute(
      'aria-checked',
      'true'
    );
    expect(screen.getByRole('menu', { name: 'Elsheria actions' })).toBeInTheDocument();
  });
});
