import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAvatarStore } from '@/state/use-avatar-store';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import ExportPanel from './ExportPanel';

afterEach(() => clearMocks());

describe('the export panel', () => {
  it('says nothing is written yet and switches the material', async () => {
    await openAvatar();
    render(<ExportPanel />);
    expect(screen.getByText('No VRM written yet.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'PBR' }));
    expect(useAvatarStore.getState().draft?.material).toBe('pbr');
    expect(useAvatarStore.getState().dirty).toBe(true);
  });

  it('shows the written file with what it still lacks', async () => {
    const calls = await openAvatar('written');
    render(<ExportPanel />);
    const last = screen.getByRole('region', { name: 'Last export' });
    expect(last).toHaveTextContent('9.4 MB');
    expect(last).toHaveTextContent('Normalised');
    expect(last).toHaveTextContent('212 vertices of cloth or hair weighted to the arms');
    expect(last).toHaveTextContent('Round trip: Reads back as a VRM with 52 humanoid bones.');
    expect(last).toHaveTextContent('exports/elsheria.vrm.report.json');
    await userEvent.click(screen.getByRole('button', { name: 'Reveal' }));
    expect(calls).toContain('workspace_reveal');
    await userEvent.click(screen.getByRole('button', { name: 'Reveal report' }));
    expect(calls.filter(c => c === 'workspace_reveal')).toHaveLength(2);
  });

  it('shows a failed round trip as a blocker in what the file lacks', async () => {
    await openAvatar('round-trip');
    render(<ExportPanel />);
    const last = screen.getByRole('region', { name: 'Last export' });
    expect(last).toHaveTextContent('Does not read back as a VRM: 2 problems');
    expect(last).toHaveTextContent('The written file does not read back as a VRM');
    expect(last).toHaveTextContent('extensionsUsed does not list VRMC_materials_mtoon.');
  });
});
