import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { TripoPresetFields } from '.';

afterEach(() => clearMocks());

const textured = { ...defaultConfig().providers.tripo.prop, texture: true, pbr: true };

describe('the Tripo preset fields', () => {
  it('turn PBR off with the texture', async () => {
    await openMockWorkspace();
    const onChange = vi.fn();
    render(<TripoPresetFields preset={textured} onChange={onChange} variant="prop" />);
    await userEvent.click(screen.getByRole('switch', { name: 'Texture' }));
    expect(onChange).toHaveBeenCalledWith({ ...textured, texture: false, pbr: false });
  });

  it('show the rig group for characters and the error under the fields', async () => {
    await openMockWorkspace();
    render(
      <TripoPresetFields
        preset={{ ...textured, rig: true }}
        onChange={() => {}}
        variant="character"
        error="Pick a model"
      />
    );
    expect(screen.getByText('Rig and animate')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Animations' })).toBeInTheDocument();
    expect(screen.getByText('Pick a model')).toBeInTheDocument();
  });

  it('leave out what Tripo Studio has no field for when sent by hand', async () => {
    await openMockWorkspace();
    render(
      <TripoPresetFields
        preset={{ ...textured, rig: true }}
        onChange={() => {}}
        variant="character"
        byHand
      />
    );
    expect(screen.getByRole('switch', { name: 'Texture' })).toBeInTheDocument();
    for (const name of ['Multi-view', 'Real-world size', 'Fix the image first', 'Smart low poly'])
      expect(screen.queryByRole('switch', { name })).toBeNull();
    expect(screen.queryByText('Rig and animate')).toBeNull();
  });
});
