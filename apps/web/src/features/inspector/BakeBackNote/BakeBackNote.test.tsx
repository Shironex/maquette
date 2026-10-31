import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import { BakeBackNote } from '.';

async function openScenario(scenario: string) {
  await openMockParts(scenario);
  useProvidersStore.setState({
    project: 'frostveil',
    jobs: await commands.providerJobs('frostveil'),
  });
}
afterEach(() => clearMocks());

describe('BakeBackNote', () => {
  it('offers the bake back a Tripo split needs, then lets the grey version be kept', async () => {
    await openScenario('split');
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

  it('asks with the price before a grey Meshy split is painted again, and spends nothing on no', async () => {
    await openScenario('grey');
    render(<BakeBackNote project="frostveil" asset="frost-lantern" />);
    expect(screen.getByText('v3 is grey for now')).toBeInTheDocument();
    const before = (await commands.providerJobs('frostveil')).length;

    await userEvent.click(screen.getByRole('button', { name: 'Re-texture...' }));
    expect(screen.getByRole('dialog')).toHaveTextContent(/About 10 credits/);
    expect(screen.getByRole('button', { name: 'Yes, re-texture' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(await commands.providerJobs('frostveil')).toHaveLength(before);

    await userEvent.click(screen.getByRole('button', { name: 'Re-texture...' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Texture prompt' }), 'frosted iron');
    await userEvent.click(screen.getByRole('button', { name: 'Yes, re-texture' }));
    expect(await screen.findByText('Re-texturing v3 on Meshy')).toBeInTheDocument();
    const jobs = await commands.providerJobs('frostveil');
    const job = jobs.find(j => j.retexture);
    expect(job?.retexture?.fromVersion).toBe(3);
    expect(job?.prompt).toBe('frosted iron');
  });
});
