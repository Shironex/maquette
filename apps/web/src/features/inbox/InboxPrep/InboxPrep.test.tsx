import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type InboxItem } from '@/ipc/client';
import { usePrepareStore } from '@/state/use-prepare-store';
import { InboxPrep } from '.';

const item = {
  id: 'desk_lamp.glb',
  name: 'Desk lamp',
  kind: 'prop',
  state: 'ready',
  report: { size: [1.25, 0.7, 0.5] },
} as unknown as InboxItem;

describe('InboxPrep', () => {
  beforeEach(() => {
    usePrepareStore.setState({ drafts: {}, suggestions: {}, previews: {} });
    vi.spyOn(commands, 'prepareSuggest').mockResolvedValue({
      size: { axis: 'height', value: 45, unit: 'cm' },
      matched: 'desk lamp',
    });
  });

  it('keeps the size in the draft until Accept and previews it on the inbox file', async () => {
    const preview = vi.spyOn(commands, 'preparePreview').mockRejectedValue({
      code: 'unavailable',
      message: 'Blender is not installed.',
    });
    render(<InboxPrep item={item} project={null} />);
    await userEvent.click(await screen.findByRole('button', { name: /Suggested: 45 cm high/ }));
    expect(usePrepareStore.getState().drafts['desk_lamp.glb']?.size).toEqual({
      axis: 'height',
      value: 45,
      unit: 'cm',
    });
    await userEvent.click(screen.getByRole('button', { name: 'Preview the numbers' }));
    expect(preview).toHaveBeenCalledWith({
      project: null,
      asset: null,
      inbox: 'desk_lamp.glb',
      plan: { size: { axis: 'height', value: 45, unit: 'cm' }, fixups: [], skipBake: null },
    });
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent('Blender is not installed.')
    );
  });
});
