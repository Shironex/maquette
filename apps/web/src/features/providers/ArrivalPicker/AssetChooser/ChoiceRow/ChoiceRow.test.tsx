import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { AssetChoice } from '@/state/downloads';
import { ChoiceRow } from '.';

const lantern: AssetChoice = {
  asset: 'lantern-post',
  name: 'Lantern post',
  picture: null,
  waiting: true,
  state: 'Waiting for its Tripo model',
};

describe('an asset in the chooser', () => {
  it('shows its name and state, says when the name looked like it, and picks on click', async () => {
    const onPick = vi.fn();
    render(<ChoiceRow id="c1" choice={lantern} active suggested onPick={onPick} />);
    const option = screen.getByRole('option', { selected: true });
    expect(option).toHaveTextContent('Lantern post');
    expect(option).toHaveTextContent('Waiting for its Tripo model · the name looked like it');
    await userEvent.click(option);
    expect(onPick).toHaveBeenCalledOnce();
  });

  it('reads plainly for an asset no send waits on', () => {
    render(
      <ChoiceRow
        id="c2"
        choice={{ ...lantern, waiting: false, state: 'Blender, v2' }}
        active={false}
        suggested={false}
        onPick={() => {}}
      />
    );
    expect(screen.getByRole('option', { selected: false })).toHaveTextContent('Blender, v2');
  });
});
