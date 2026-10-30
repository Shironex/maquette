import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAvatarStore } from '@/state/use-avatar-store';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import MetaForm from './MetaForm';

afterEach(() => clearMocks());

describe('the meta form', () => {
  it('starts from the defaults, takes authors by commas and saves', async () => {
    const calls = await openAvatar();
    render(<MetaForm />);
    expect(screen.getByText('Name at least one author.')).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText('Authors'), 'Ana, Studio North');
    expect(useAvatarStore.getState().draft?.meta.authors).toEqual(['Ana', 'Studio North']);
    await userEvent.selectOptions(screen.getByLabelText('Commercial use'), 'corporation');
    await userEvent.click(screen.getByLabelText('Others may share the file'));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(calls).toContain('avatar_save');
    expect(useAvatarStore.getState().view?.spec.meta).toMatchObject({
      authors: ['Ana', 'Studio North'],
      commercialUsage: 'corporation',
      allowRedistribution: true,
    });
    expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });
});
