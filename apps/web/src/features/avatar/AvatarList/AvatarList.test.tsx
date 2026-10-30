import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import AvatarList from './AvatarList';

afterEach(() => clearMocks());

describe('the avatar list', () => {
  it("lists the project's characters and marks the open one", async () => {
    await openAvatar();
    render(<AvatarList />);
    const row = screen.getByRole('button', { name: /Elsheria/ });
    expect(row).toHaveAttribute('aria-current', 'true');
    expect(row).toHaveTextContent('v2');
    expect(screen.queryByRole('button', { name: /Veil gate/ })).not.toBeInTheDocument();
  });
});
