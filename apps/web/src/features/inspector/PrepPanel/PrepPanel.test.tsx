import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands, type InspectModel } from '@/ipc/client';
import { usePrepareStore } from '@/state/use-prepare-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import PrepPanel from './PrepPanel';

let character: InspectModel;

beforeEach(async () => {
  await openMockWorkspace();
  const view = await commands.inspectProject('moon-island');
  character = view.models.find(m => m.model === 'character')!;
});
afterEach(() => {
  clearMocks();
  usePrepareStore.setState({ root: null, assets: {}, failed: {}, previews: {}, saving: null });
});

describe('the real size panel', () => {
  it('saves a fix-up at once and previews it on the newest version', async () => {
    render(<PrepPanel project="moon-island" model={character} />);
    const panel = await screen.findByRole('region', { name: 'Real size and fix-ups' });
    await within(panel).findByText(/Measured/);
    await userEvent.click(within(panel).getByRole('button', { name: 'Decimate' }));
    await waitFor(async () =>
      expect((await commands.prepareGet('moon-island')).models.character?.fixups).toEqual([
        { op: 'decimate', triangles: 60_000 },
      ])
    );
    expect(within(panel).getByRole('list')).toHaveTextContent('Decimate');
    await userEvent.click(within(panel).getByRole('button', { name: 'Preview the numbers' }));
    const table = await within(panel).findByRole('table', { name: 'Before and after the fix-ups' });
    expect(within(table).getByRole('row', { name: /Triangles/ })).toHaveTextContent('60,000');
  });
});
