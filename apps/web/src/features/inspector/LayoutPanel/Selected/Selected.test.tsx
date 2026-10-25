import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { LayoutView } from '@/ipc/client';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import Selected from './Selected';

let view: LayoutView;

beforeEach(async () => {
  await openMockWorkspace();
  await useLayoutStore.getState().load('moon-island');
  view = useLayoutStore.getState().view!;
});
afterEach(() => {
  clearMocks();
  useLayoutStore.setState({ view: null, project: null, selection: null });
});

describe('the selected item', () => {
  it('explains how to select when nothing is', () => {
    render(<Selected view={view} layout={useLayoutStore.getState().draft} selection={null} />);
    expect(screen.getByText(/Click a model or a camera in the view/)).toBeInTheDocument();
  });

  it("shows a model's placement fields", () => {
    const layout = useLayoutStore.getState().draft;
    render(<Selected view={view} layout={layout} selection={{ kind: 'model', id: 'island' }} />);
    expect(screen.getByText('Island')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Position in m' })).toBeInTheDocument();
  });

  it("shows a stop's camera fields, and the hint for one that is gone", () => {
    const layout = useLayoutStore.getState().draft;
    const { rerender } = render(
      <Selected view={view} layout={layout} selection={{ kind: 'stop', id: 'sakura' }} />
    );
    expect(screen.getByRole('group', { name: 'Looks at in m' })).toBeInTheDocument();
    rerender(<Selected view={view} layout={layout} selection={{ kind: 'stop', id: 'gone' }} />);
    expect(screen.getByText(/Click a model or a camera in the view/)).toBeInTheDocument();
  });
});
