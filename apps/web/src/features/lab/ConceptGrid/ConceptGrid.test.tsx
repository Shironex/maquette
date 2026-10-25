import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { ConceptGrid } from '.';

const view: LabView = {
  project: 'moon-island',
  projectDir: '/w/moon-island',
  directions: labDirections(false),
  jobs: [],
  refs: [],
  concurrency: 4,
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('the concept grid', () => {
  it('explains the first steps before a direction is selected', () => {
    render(<ConceptGrid view={view} />);
    expect(screen.getByText('No concepts yet')).toBeVisible();
    expect(screen.getByText('Write the brief')).toBeVisible();
  });

  it('shows the selected direction', () => {
    useLabStore.setState({ selected: 'late-train' });
    render(<ConceptGrid view={view} />);
    expect(screen.getByRole('region', { name: 'Late train concepts' })).toBeVisible();
  });
});
