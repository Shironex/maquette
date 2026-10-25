import { render, screen } from '@testing-library/react';
import { clearMocks } from '@tauri-apps/api/mocks';
import { act } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { openAgentSession } from '../openAgentSession';
import { ApprovalPrompt } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null });
});

describe('the approval prompt', () => {
  it('floats the waiting approvals over every view but Settings', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    expect(screen.getAllByRole('alertdialog')).toHaveLength(2);
    act(() => useAppStore.setState({ view: 'settings' }));
    expect(screen.queryByRole('alertdialog')).toBeNull();
  });

  it('renders nothing with no approvals', async () => {
    await openAgentSession(null);
    const { container } = render(<ApprovalPrompt />);
    expect(container).toBeEmptyDOMElement();
  });
});
