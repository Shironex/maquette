import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { WorkspaceConfig } from '@/ipc/client';
import { defaultConfig } from '@/ipc/mock/fixtures';
import AvatarSection from './AvatarSection';

describe('the avatar defaults', () => {
  it("start at VRM's strictest licence and change one field at a time", async () => {
    const onChange = vi.fn();
    render(<AvatarSection active config={defaultConfig()} onChange={onChange} />);
    expect(screen.getByRole('region', { name: /Avatar defaults/ })).toBeInTheDocument();
    expect(screen.getByLabelText('Who may play it')).toHaveValue('onlyAuthor');
    await userEvent.type(screen.getByLabelText('Author'), 'A');
    expect((onChange.mock.calls.at(-1)?.[0] as WorkspaceConfig).avatar?.author).toBe('A');
    await userEvent.click(screen.getByRole('button', { name: 'PBR' }));
    expect((onChange.mock.calls.at(-1)?.[0] as WorkspaceConfig).avatar?.material).toBe('pbr');
  });
});
