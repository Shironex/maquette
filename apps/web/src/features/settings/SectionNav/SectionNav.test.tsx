import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { SectionNav } from '.';

afterEach(() => clearMocks());

describe('the settings nav', () => {
  it('marks the section in view and jumps to another', async () => {
    await openMockWorkspace();
    const onSelect = vi.fn();
    render(<SectionNav active="budgets" onSelect={onSelect} />);
    expect(screen.getByRole('button', { name: 'Budgets and compression' })).toHaveAttribute(
      'aria-current',
      'true'
    );
    expect(screen.getByRole('button', { name: 'Workspace' })).not.toHaveAttribute('aria-current');
    await userEvent.click(screen.getByRole('button', { name: 'Agents' }));
    expect(onSelect).toHaveBeenCalledWith('agents');
  });

  it('lists the open project under This project', async () => {
    await openMockWorkspace();
    const onSelect = vi.fn();
    render(<SectionNav active="workspace" onSelect={onSelect} />);
    expect(screen.getByText('This project')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Moon Island' }));
    expect(onSelect).toHaveBeenCalledWith('project');
  });
});
