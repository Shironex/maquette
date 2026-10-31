import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { GAME_TEMPLATES } from '@/ipc/mock/game-templates';
import { emptyProjectFields, newProjectRequest } from '@/state/onboarding';
import { GameTemplatePicker } from '.';

vi.mock('@/ipc/client', async importOriginal => ({
  ...(await importOriginal<typeof import('@/ipc/client')>()),
  commands: { gameTemplateList: () => Promise.resolve(GAME_TEMPLATES) },
}));

const fields = { ...emptyProjectFields('Crypt'), target: 'game' as const };

describe('GameTemplatePicker', () => {
  it('starts on no template and fills the format when one is picked', async () => {
    const onChange = vi.fn();
    render(<GameTemplatePicker value={fields} onChange={onChange} />);
    expect(screen.getByRole('button', { name: /^No template/ })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(await screen.findByRole('button', { name: /^Level kit, \.obj/ }));
    expect(onChange).toHaveBeenCalledWith({ gameTemplate: 'level-kit-obj', gameFormat: 'obj' });
  });

  it('says what it sets, and that the Model format below wins', async () => {
    render(
      <GameTemplatePicker
        value={{ ...fields, gameTemplate: 'character', gameFormat: 'obj' }}
        onChange={vi.fn()}
      />
    );
    await screen.findByText(/characters\/ at 60k triangles, 2048 px/);
    expect(screen.getByText(/Adds no models and no game code/)).toBeInTheDocument();
    expect(screen.getByText(/The Model format below wins/)).toBeInTheDocument();
  });

  it('goes into the new project request', () => {
    const request = newProjectRequest({ ...fields, gameTemplate: 'prop-set' });
    expect(request.game?.template).toBe('prop-set');
    expect(newProjectRequest(fields).game?.template).toBeNull();
  });
});
