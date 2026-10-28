import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import { BakeBackNote } from '.';

beforeEach(async () => {
  await openMockParts('split');
  useProvidersStore.setState({
    project: 'frostveil',
    jobs: await commands.providerJobs('frostveil'),
  });
});
afterEach(() => clearMocks());

describe('BakeBackNote', () => {
  it('offers the bake back a Tripo split needs, then lets the grey version be kept', async () => {
    render(<BakeBackNote project="frostveil" asset="frost-lantern" />);
    expect(screen.getByText('Tripo did not keep the texture')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Bake back' }));
    expect(await screen.findByText('Texture baked back as v4')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Keep v3' }));
    const lantern = useAppStore
      .getState()
      .workspace?.project?.assets?.find(a => a.id === 'frost-lantern');
    expect(lantern?.versions.at(-1)?.version).toBe(3);
  });
});
