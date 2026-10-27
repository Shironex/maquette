import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { LayoutModel } from '@/ipc/client';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import ModelDetails from './ModelDetails';

let shop: LayoutModel;

beforeEach(async () => {
  await openMockWorkspace();
  await useLayoutStore.getState().load('frostveil');
  shop = useLayoutStore.getState().view!.models.find(m => m.asset === 'observatory-arch')!;
});
afterEach(() => {
  clearMocks();
  useLayoutStore.setState({ view: null, project: null, selection: null });
});

function details() {
  return render(<ModelDetails model={shop} layout={useLayoutStore.getState().draft} />);
}

describe('the model details', () => {
  it('names the model with its size at its scale and how it was measured', () => {
    details();
    expect(screen.getByText('Observatory arch')).toBeInTheDocument();
    expect(screen.getByText(/118 × 156 × 98\.4 cm/)).toBeInTheDocument();
    expect(screen.getByText('Uniform. Measured from the raw export.')).toBeInTheDocument();
    expect(screen.getByLabelText(/^Scale/)).toHaveValue('1.200');
  });

  it('moves it and resets its turn and scale', async () => {
    const { rerender } = details();
    await userEvent.click(screen.getByRole('button', { name: 'Reset turn and scale' }));
    expect(useLayoutStore.getState().draft.models['observatory-arch']).toMatchObject({
      rotation: [0, 0, 0],
      scale: 1,
    });
    rerender(<ModelDetails model={shop} layout={useLayoutStore.getState().draft} />);
    const x = screen.getAllByLabelText('X')[0]!;
    await userEvent.clear(x);
    await userEvent.type(x, '-1{Enter}');
    expect(useLayoutStore.getState().draft.models['observatory-arch']?.position[0]).toBe(-1);
  });

  it('drops it onto the desk', async () => {
    details();
    await userEvent.click(screen.getByRole('button', { name: 'Drop to desk' }));
    expect(useLayoutStore.getState().draft.models['observatory-arch']?.position[1]).toBeCloseTo(0);
  });
});
