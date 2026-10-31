import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import OnlyBones from './OnlyBones';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null });
});

describe('bones only on the model', () => {
  it('rolls chains into a row, says what carries them and keeps a choice', async () => {
    await openMockLibrary('demo');
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['standing-dodge-left'],
      step: 'map',
    });
    render(<OnlyBones name="Elsheria" />);
    expect(screen.getByText('cloak_back_c_01 to 04')).toBeInTheDocument();
    expect(screen.getByText(/Fingers · \d+ bones stay at rest/)).toBeInTheDocument();
    const select = screen.getByRole('combobox', { name: 'What cloak_back_c_01 to 04 follows' });
    expect(select).toHaveTextContent('Follow mixamorig:Hips, above it');
    useRetargetStore.getState().setFollow(['cloak_back_c_01'], 'mixamorig:Spine');
    expect(useRetargetStore.getState().sheet!.follow).toEqual([
      { bone: 'cloak_back_c_01', follows: 'mixamorig:Spine' },
    ]);
  });
});
