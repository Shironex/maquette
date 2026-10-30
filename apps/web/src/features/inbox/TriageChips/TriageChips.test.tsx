import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useInboxStore } from '@/state/use-inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { TriageChips } from '.';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'triage');
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
});

describe('the inbox filter chips', () => {
  it('counts each group and picks one', async () => {
    const onChange = vi.fn();
    const items = useInboxStore.getState().snapshot!.items;
    render(<TriageChips items={items} value="all" onChange={onChange} />);
    expect(screen.getByRole('button', { name: 'All 12' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Pass 6' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Unreadable 1' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Replaced 2' }));
    expect(onChange).toHaveBeenCalledWith('replaced');
  });

  it('leaves out an empty group unless it is the one picked', () => {
    const items = useInboxStore.getState().snapshot!.items.filter(i => i.state !== 'failed');
    const { rerender } = render(<TriageChips items={items} value="all" onChange={() => {}} />);
    expect(screen.queryByRole('button', { name: /Unreadable/ })).toBeNull();
    rerender(<TriageChips items={items} value="unreadable" onChange={() => {}} />);
    expect(screen.getByRole('button', { name: 'Unreadable 0' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
  });
});
