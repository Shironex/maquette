import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { LocateButton } from '.';

afterEach(cleanup);

describe('the locate button', () => {
  it('names the tool and hands its id to onLocate', async () => {
    const onLocate = vi.fn(async () => {});
    render(
      <LocateButton
        tool="ffmpeg"
        label="ffmpeg"
        quiet={false}
        disabled={false}
        onLocate={onLocate}
        fixedWidth
      />
    );
    const button = screen.getByRole('button', { name: 'Locate ffmpeg' });
    expect(button).toHaveClass('w-[72px]');
    await userEvent.click(button);
    expect(onLocate).toHaveBeenCalledWith('ffmpeg');
  });

  it('does nothing while disabled', async () => {
    const onLocate = vi.fn(async () => {});
    render(
      <LocateButton
        tool="node"
        label="Node"
        quiet
        disabled
        onLocate={onLocate}
        fixedWidth={false}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: 'Locate Node' }));
    expect(onLocate).not.toHaveBeenCalled();
  });
});
