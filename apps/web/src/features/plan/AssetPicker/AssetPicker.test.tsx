import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openPlans } from '@/test/open-plan';
import { AssetPicker } from '.';

afterEach(clearMocks);

const renderPicker = async (linked = { assets: ['elsheria'], directions: [] as string[] }) => {
  await openPlans();
  const onAdd = vi.fn();
  const onClose = vi.fn();
  render(<AssetPicker linked={linked} onAdd={onAdd} onClose={onClose} />);
  return { onAdd, onClose };
};

describe('AssetPicker', () => {
  it('lists the Board and the Lab, marking what the canvas links', async () => {
    await renderPicker();
    const board = screen.getByRole('region', { name: 'On the Board' });
    expect(within(board).getByText('Elsheria')).toBeInTheDocument();
    expect(within(board).getByText(/on this canvas/)).toBeInTheDocument();
    const lab = screen.getByRole('region', { name: 'In the Lab' });
    expect(await within(lab).findByText('Frozen lake arena')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Search' })).toHaveFocus();
  });

  it('searches both lists by name', async () => {
    await renderPicker();
    await screen.findByText('Frozen lake arena');
    const search = screen.getByRole('textbox', { name: 'Search' });
    await userEvent.type(search, 'FROZEN LAKE');
    expect(screen.queryByText('Elsheria')).toBeNull();
    expect(screen.getByText('Frozen lake arena')).toBeInTheDocument();
    await userEvent.clear(search);
    await userEvent.type(search, 'nothing like this');
    expect(screen.getByText('No asset by that name')).toBeInTheDocument();
    expect(screen.getByText('No direction by that name')).toBeInTheDocument();
  });

  it('adds only the kind and id of each ticked row', async () => {
    const { onAdd, onClose } = await renderPicker();
    const add = screen.getByRole('button', { name: 'Add' });
    expect(add).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(await screen.findByRole('checkbox', { name: /Veil gate kit/ }));
    const board = screen.getByRole('region', { name: 'On the Board' });
    await userEvent.click(within(board).getByRole('checkbox', { name: /^Elsheria/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Add 2' }));
    expect(onAdd).toHaveBeenCalledWith([
      { kind: 'asset', id: 'elsheria' },
      { kind: 'direction', id: expect.any(String) },
    ]);
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onClose).toHaveBeenCalled();
  });
});
