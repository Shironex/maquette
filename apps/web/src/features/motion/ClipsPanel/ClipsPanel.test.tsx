import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import ClipsPanel from './ClipsPanel';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => clearMocks());

describe('the clips panel', () => {
  it('lists the clips with their ship switches and the rig check', async () => {
    await openMotion();
    render(<ClipsPanel />);
    expect(screen.getByText('Clips · 3')).toBeInTheDocument();
    const wave = screen.getByRole('switch', { name: 'Ship Wave' });
    expect(wave).toBeChecked();
    fireEvent.click(wave);
    expect(useMotionStore.getState().draft.clipsOff['clay-figure']).toEqual(['Wave']);
    expect(screen.getByText('20 bones, humanoid rig')).toBeInTheDocument();
  });

  it('selects a clip and shows its numbers', async () => {
    await openMotion();
    render(<ClipsPanel />);
    fireEvent.click(screen.getByRole('button', { name: /Walk loop/ }));
    expect(useMotionStore.getState().clip).toBe('Walk loop');
    expect(screen.getByText('ships as Walk loop')).toBeInTheDocument();
    expect(screen.getByText('Root motion, 1.42 m')).toBeInTheDocument();
  });

  it('offers the ways to add a clip, library clips only with a rig', async () => {
    await openMotion(undefined, 'turntable');
    render(<ClipsPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Add clip' }));
    expect(screen.getByRole('menuitem', { name: 'Retarget a library clip' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    fireEvent.click(screen.getByRole('menuitem', { name: 'From a .glb in the Inbox' }));
    expect(useAppStore.getState().view).toBe('inbox');
  });
});

describe('the clips panel with older starters', () => {
  it('says every clip ships and locks the switches', async () => {
    await openMotion();
    const view = useMotionStore.getState().view!;
    useMotionStore.setState({ view: { ...view, clipsOffSupported: false } });
    render(<ClipsPanel />);
    expect(screen.getByText('Every clip ships for now')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Ship Wave' })).toBeDisabled();
  });
});
