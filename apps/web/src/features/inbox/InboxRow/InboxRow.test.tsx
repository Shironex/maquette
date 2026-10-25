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
});
