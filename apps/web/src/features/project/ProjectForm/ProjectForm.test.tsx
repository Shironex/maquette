import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { emptyProjectFields, type ProjectFields } from '@/state/onboarding';
import { ProjectForm } from '.';

afterEach(cleanup);

const actions = () => ({
  checkRepo: vi.fn(async () => {}),
  browseRepo: vi.fn(async () => {}),
  browseSource: vi.fn(async () => {}),
});

function renderForm(value: Partial<ProjectFields> = {}, locked = false) {
  const onChange = vi.fn();
  const acts = actions();
  render(
    <ProjectForm
      value={{ ...emptyProjectFields(), ...value }}
      onChange={onChange}
      actions={acts}
      locked={locked}
    />
  );
  return { onChange, acts };
}

describe('the project form', () => {
  it('offers the templates and switches to an import', async () => {
    const { onChange } = renderForm();
    expect(screen.getByRole('group', { name: 'Templates' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Import a folder' }));
    expect(onChange).toHaveBeenCalledWith({ mode: 'import' });
  });

  it('only keeps the name and repo when the project exists', () => {
    renderForm({}, true);
    expect(screen.getByText(/The project exists/)).toBeVisible();
    expect(screen.queryByRole('group', { name: 'How to start' })).toBeNull();
    expect(screen.getByLabelText('Project name')).toBeVisible();
  });

  it('asks for a name once the field is left empty', async () => {
    renderForm();
    expect(screen.queryByText('Give the project a name.')).toBeNull();
    await userEvent.click(screen.getByLabelText('Project name'));
    await userEvent.tab();
    expect(screen.getByText('Give the project a name.')).toBeVisible();
  });

  it('shows what the site repo check found', () => {
    renderForm({
      siteRepo: '~/site',
      repo: { path: '/site', exists: true, isGit: false, hasPublicDir: true },
    });
    expect(screen.getByText('Not a git repo.')).toBeVisible();
    expect(screen.getByText(/Ship still writes there/)).toBeVisible();
  });
});
