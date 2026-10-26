import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import NoClipsOptions from './NoClipsOptions';

afterEach(() => clearMocks());

describe('the ways to a first clip', () => {
  it('retargets onto a rig, opens the Inbox or adds a script', async () => {
    await openMotion('rest');
    render(<NoClipsOptions />);
    fireEvent.click(screen.getByRole('button', { name: 'Retarget a clip' }));
    expect(useMotionStore.getState().boneMap).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Add script' }));
    expect(useAppStore.getState().view).toBe('board');
    expect(useBuiltStore.getState().adding).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Open the Inbox' }));
    expect(useAppStore.getState().view).toBe('inbox');
  });
});
