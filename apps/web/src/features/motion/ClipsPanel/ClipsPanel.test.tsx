import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useBakedStore } from '@/state/use-baked-store';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openGameMotion, openMotion } from '@/test/open-motion';
import ClipsPanel from './ClipsPanel';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => clearMocks());

describe('the clips panel in a game', () => {
  it('shows every clip name in full, with no scene budget', async () => {
    await openGameMotion();
    render(<ClipsPanel />);
    expect(screen.getByText('Clips · 187')).toBeInTheDocument();
    const name = 'Combat_Run_Forward_Combat_Stance_01';
    const row = screen.getByRole('button', { name: new RegExp(name) });
    expect(row).toHaveAttribute('title', name);
    expect(within(row).getByText(name)).toHaveClass('truncate');
    expect(screen.queryByText('Over')).toBeNull();
    expect(screen.queryByText(/scene/i)).toBeNull();
    expect(screen.getByText(/Game export keeps the clips switched on/)).toBeInTheDocument();
  });

  it('saves a switched off clip at once, there being no Save motion', async () => {
    await openGameMotion();
    render(<ClipsPanel />);
    const name = 'Combat_Run_Forward_Combat_Stance_01';
    fireEvent.click(screen.getByRole('switch', { name: `Ship ${name}` }));
    const asset = useMotionStore.getState().asset!;
    await waitFor(() => expect(useMotionStore.getState().view?.clipsOff[asset]).toEqual([name]));
    expect(useMotionStore.getState().draft.clipsOff[asset]).toEqual([name]);
  });

  it('finds clips by the words of their names', async () => {
    await openGameMotion();
    render(<ClipsPanel />);
    const search = screen.getByRole('searchbox', { name: 'Search clips' });
    fireEvent.change(search, { target: { value: 'combat_parry overhead 02' } });
    expect(screen.getByText('Clips · 1 of 187')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Combat_Parry_Riposte_Overhead_02/ })).toBeVisible();
    fireEvent.change(search, { target: { value: 'nothing like it' } });
    expect(screen.getByText('No clip name has all of those words.')).toBeInTheDocument();
  });
});

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
    expect(screen.getByRole('menuitem', { name: 'Add from library' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(screen.getByRole('menuitem', { name: 'Save clips to library' })).toHaveAttribute(
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

describe('a model with library clips baked in', () => {
  it('groups them apart from its own and says which the last bake left out', async () => {
    await openMotion();
    const asset = useMotionStore.getState().asset!;
    const setting = {
      clip: 'standing-dodge-left',
      name: 'Wave',
      start: 4,
      end: 34,
      loops: false,
      rootMotion: true,
      speed: 1,
      pinFeet: false,
    };
    const version = useMotionStore.getState().view!.models.find(m => m.asset === asset)!.version;
    useBakedStore.setState({
      project: 'frostveil',
      record: {
        version: 1,
        assets: {
          [asset]: {
            map: [],
            follow: [],
            baked: [
              {
                setting,
                revision: 1,
                libraryName: 'Standing Dodge Left',
                version,
                libraryFrames: 38,
              },
            ],
            reviewVersion: version,
            review: {
              clips: [
                {
                  clip: 'crouch',
                  name: 'crouch_guard',
                  status: 'failed',
                  note: 'Failed',
                  frames: 44,
                  travel: null,
                  slideCm: 0,
                  slideFrames: null,
                  unpinnedCm: null,
                },
              ],
              seconds: 5,
              driven: 20,
              atRest: [],
              alignedDeg: 47,
              hipsScale: 1,
            },
          },
        },
      },
    });
    render(<ClipsPanel />);
    expect(screen.getByText('In the model file · 2')).toBeInTheDocument();
    expect(screen.getByText(/^From the library · 1/)).toBeInTheDocument();
    expect(screen.getByText(`new in v${version}`)).toBeInTheDocument();
    expect(screen.getByText(/crouch_guard failed in review and was left out/)).toBeInTheDocument();
    useBakedStore.setState({ project: null, record: null });
  });
});
