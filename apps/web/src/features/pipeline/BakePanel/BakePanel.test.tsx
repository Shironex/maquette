import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { BakeView } from '@/ipc/client';
import { BakePanel } from '.';
import { bakePatch, costText, relativeCost } from './BakePanel.hooks';

const bake: BakeView = {
  settings: { size: 1024, samples: 32, denoise: true },
  canDenoise: true,
  script: 'current',
  skipped: ['desk-globe', 'pencil-cup'],
};

describe('the bake settings', () => {
  it('saves only what changed and says what it costs', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(<BakePanel bake={bake} busy={false} onSave={onSave} />);
    expect(
      screen.getByText('2 models skip the bake and keep their own lighting.')
    ).toBeInTheDocument();
    const apply = screen.getByRole('button', { name: 'Apply to the bake' });
    expect(apply).toBeDisabled();

    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Lightmap size' }),
      '2048 px'
    );
    expect(screen.getByText('About 4x the default bake time')).toBeInTheDocument();
    await userEvent.click(apply);
    expect(onSave).toHaveBeenCalledWith({ size: 2048, samples: null, denoise: null });
  });

  it('keeps denoise off and says why when the script cannot denoise', () => {
    const older: BakeView = {
      ...bake,
      settings: { size: 512, samples: 16, denoise: false },
      canDenoise: false,
      script: 'outdated',
      skipped: [],
    };
    render(<BakePanel bake={older} busy={false} onSave={vi.fn()} />);
    expect(screen.getByRole('switch', { name: 'Denoise' })).toBeDisabled();
    expect(screen.getByText(/cannot denoise/)).toBeInTheDocument();
    expect(screen.getByText('Every model is baked.')).toBeInTheDocument();
  });

  it('weighs the time by texels and samples', () => {
    expect(relativeCost({ size: 512, samples: 16, denoise: false })).toBe(0.125);
    expect(costText({ size: 1024, samples: 32, denoise: false })).toBe('The default bake time');
    expect(costText({ size: 4096, samples: 64, denoise: true })).toBe(
      'About 32x the default bake time'
    );
    expect(bakePatch(bake.settings, { ...bake.settings, denoise: false })).toEqual({
      size: null,
      samples: null,
      denoise: false,
    });
  });
});
