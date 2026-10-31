import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { RenderPlan } from '@/ipc/client';
import { RENDER_DEFAULTS } from '@/state/renders';
import { useRenderStore } from '@/state/use-render-store';
import ContactPreview from './ContactPreview';

afterEach(() => useRenderStore.setState({ plan: null, rows: {}, ticked: [], record: null }));

describe('the contact sheet of a selection', () => {
  it('names each asset and lists what goes where', () => {
    useRenderStore.setState({
      settings: RENDER_DEFAULTS,
      ticked: ['a'],
      rows: { a: ['clay', 'turntable'] },
      plan: {
        label: 'of X',
        dir: '/w/projects/x/exports/renders',
        skipped: [],
        targets: [{ key: 'a', name: 'Pumpkin', folder: 'pumpkin', thumbnail: null }],
      } as unknown as RenderPlan,
    });
    render(<ContactPreview />);
    expect(screen.getByText('pumpkin')).toBeVisible();
    expect(screen.getByText('pumpkin/ clay.png, turntable.webp')).toBeVisible();
    expect(screen.getByText('exports/renders/')).toBeVisible();
  });
});
