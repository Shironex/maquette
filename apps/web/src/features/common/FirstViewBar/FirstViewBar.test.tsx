import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { FirstView } from '@/ipc/client';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { FirstViewBar } from '.';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null });
});

async function withFirstView(firstView: FirstView) {
  await openMockWorkspace();
  await usePipelineStore.getState().load('moon-island');
  const view = usePipelineStore.getState().view!;
  usePipelineStore.setState({ view: { ...view, firstView } });
}

describe('the first view bar', () => {
  it('shows nothing before the pipeline loads', () => {
    const { container } = render(<FirstViewBar />);
    expect(container).toBeEmptyDOMElement();
  });

  it('names the first stop and says what it loads', async () => {
    await withFirstView({
      stop: 'Porch',
      custom: false,
      models: [],
      bytes: 1_500_000,
      targetMb: 3,
    });
    render(<FirstViewBar className="w-40" />);
    expect(screen.getByText('First view · Porch')).toBeVisible();
    expect(screen.getByTitle(/^First view \(Porch\): 0 models in its frame/)).toHaveClass('w-40');
  });

  it('says when there is no camera stop yet', async () => {
    await withFirstView({ stop: null, custom: false, models: [], bytes: null, targetMb: 3 });
    render(<FirstViewBar />);
    expect(screen.getByText('First view · no stop yet')).toBeVisible();
  });
});
