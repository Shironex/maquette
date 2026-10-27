import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labRefs } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { References } from '.';

const refs = labRefs();
const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions: [],
  jobs: [],
  refs,
  concurrency: 4,
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('the references', () => {
  it('shows a chip per draft ref and removes one', async () => {
    const removeRef = vi.fn();
    const first = refs[0]!.file;
    act(() => useLabStore.getState().setDraft({ refs: [first] }));
    useLabStore.setState({ removeRef });
    render(<References view={view} />);
    const name = first.slice(first.lastIndexOf('/') + 1);
    await userEvent.click(screen.getByRole('button', { name: `Remove ${name}` }));
    expect(removeRef).toHaveBeenCalledWith(first);
  });

  it('offers the unused refs/ images in the Add menu', async () => {
    const addRefs = vi.fn(async () => {});
    useLabStore.setState({ addRefs });
    render(<References view={view} />);
    await userEvent.click(screen.getByRole('button', { name: 'Add reference image' }));
    expect(screen.getByText('From refs/')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: refs[1]!.name }));
    expect(addRefs).toHaveBeenCalledWith([refs[1]!.file]);
    expect(screen.queryByText('From refs/')).toBeNull();
  });

  it('says when refs/ has nothing left to add', async () => {
    render(<References view={{ ...view, refs: [] }} />);
    await userEvent.click(screen.getByRole('button', { name: 'Add reference image' }));
    expect(screen.getByText(/Nothing in refs\/ yet/)).toBeVisible();
  });
});
