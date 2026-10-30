import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { VrmStageProps, VrmSummary } from '@/features/viewer/VrmStage';
import { useAvatarStore } from '@/state/use-avatar-store';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import PreviewPanel from './PreviewPanel';

const stage = vi.hoisted(() => ({
  props: null as VrmStageProps | null,
  summary: null as VrmSummary | null,
  told: new Set<string>(),
}));

/** Stands in for the lazy stage: says once per file that it loaded, or failed. */
vi.mock('@/features/viewer/VrmStage', () => ({
  default: function FakeStage(props: { label: string } & Partial<VrmStageProps>) {
    stage.props = props as VrmStageProps;
    const { url = '', onLoaded, onError } = props;
    if (!stage.told.has(url)) {
      stage.told.add(url);
      queueMicrotask(() => {
        if (stage.summary) onLoaded?.(stage.summary);
        else onError?.('The file ends early.');
      });
    }
    return <div role="img" aria-label={props.label} />;
  },
}));

const SUMMARY: VrmSummary = {
  spec: '1.0',
  bones: 52,
  missing: [],
  springs: 0,
  expressions: 0,
  lookAt: 'expression',
  lookAtMoves: false,
};

afterEach(() => {
  clearMocks();
  stage.props = null;
  stage.summary = null;
  stage.told.clear();
});

describe('the preview panel', () => {
  it('points to the Export tab while no vrm is written', async () => {
    await openAvatar();
    useAvatarStore.setState({ tab: 'preview' });
    render(<PreviewPanel />);
    expect(screen.getByText('No VRM written yet')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Go to Export' }));
    expect(useAvatarStore.getState().tab).toBe('export');
  });

  it('loads the written vrm and says what it holds and why the eyes stay still', async () => {
    stage.summary = SUMMARY;
    await openAvatar('written');
    render(<PreviewPanel />);
    expect(await screen.findByRole('img', { name: 'Elsheria as a VRM' })).toBeInTheDocument();
    expect(stage.props).toMatchObject({ armsDown: true, lookAt: true });
    expect(await screen.findByText('52 of 55 bones')).toBeInTheDocument();
    expect(screen.getByText('15 of 15 required')).toBeInTheDocument();
    expect(screen.getByText('No spring bones')).toBeInTheDocument();
    expect(screen.getByText('The eyes stay still')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Look at pointer' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.click(screen.getByRole('button', { name: 'T-pose' }));
    await waitFor(() => expect(stage.props?.armsDown).toBe(false));
    await userEvent.click(screen.getByRole('button', { name: 'Breathe' }));
    expect(stage.props?.breathe).toBe(false);
  });

  it('names the required bones three-vrm did not find', async () => {
    stage.summary = { ...SUMMARY, lookAt: 'bone', lookAtMoves: true, missing: ['leftHand'] };
    await openAvatar('written');
    render(<PreviewPanel />);
    expect(await screen.findByText('three-vrm found no leftHand.')).toBeInTheDocument();
    expect(screen.queryByText('The eyes stay still')).not.toBeInTheDocument();
  });

  it('shows a file that does not load, and a failed round trip', async () => {
    await openAvatar('round-trip');
    render(<PreviewPanel />);
    expect(await screen.findByText('The VRM could not be loaded')).toBeInTheDocument();
    expect(screen.getByText('The written file did not read back as a VRM')).toBeInTheDocument();
  });
});
