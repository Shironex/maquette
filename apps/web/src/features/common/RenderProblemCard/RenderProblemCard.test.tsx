import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import RenderProblemCard from './RenderProblemCard';

describe('a problem with a render', () => {
  it('names the tool that is missing and offers the way out', async () => {
    const open = vi.fn();
    render(
      <RenderProblemCard
        name="Crystal lantern"
        style="turntable"
        problem={{
          kind: 'missing-tool',
          message: 'A GIF needs ffmpeg. Set its path in Settings, Toolchain.',
          webpBytes: null,
        }}
        actions={[{ label: 'Open Toolchain', icon: 'wrench', onClick: open }]}
      />
    );
    expect(screen.getByRole('heading', { name: 'ffmpeg was not found' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Open Toolchain' }));
    expect(open).toHaveBeenCalledOnce();
  });

  it('says what a GIF came out at', () => {
    render(
      <RenderProblemCard
        name="Crystal lantern"
        style="turntable"
        bytes={6_800_000}
        problem={{ kind: 'over-target', message: 'Over the 2 MB target.', webpBytes: 1_100_000 }}
      />
    );
    expect(screen.getByRole('heading', { name: 'Turntable GIF came out at 6.8 MB' })).toBeVisible();
  });
});
