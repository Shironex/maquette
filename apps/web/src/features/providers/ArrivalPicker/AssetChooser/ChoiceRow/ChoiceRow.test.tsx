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
  score: 0.82,
  fit: '82% alike',
};

describe('an asset in the chooser', () => {
  it('shows its name, state and fit, says when it is the suggestion, and picks on click', async () => {
    const onPick = vi.fn();
    render(<ChoiceRow id="c1" choice={lantern} active suggested onPick={onPick} />);
    const option = screen.getByRole('option', { selected: true });
    expect(option).toHaveTextContent('Lantern post');
    expect(option).toHaveTextContent(
      'Waiting for its Tripo model · the watcher thinks it is this one'
    );
    expect(option).toHaveTextContent('82% alike');
    await userEvent.click(option);
    expect(onPick).toHaveBeenCalledOnce();
  });

  it('reads plainly for an asset no send waits on', () => {
    render(
      <ChoiceRow
        id="c2"
        choice={{ ...lantern, waiting: false, state: 'Blender, v2', score: null, fit: null }}
        active={false}
        suggested={false}
        onPick={() => {}}
      />
    );
    const option = screen.getByRole('option', { selected: false });
    expect(option).toHaveTextContent('Blender, v2');
    expect(option).not.toHaveTextContent('alike');
  });
});
