import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { MeshyPresetFields } from '.';

afterEach(() => clearMocks());

const base = defaultConfig().providers.meshy.prop;

describe('the Meshy preset fields', () => {
  it('lock remesh and topology for a Smart Topology model', async () => {
    await openMockWorkspace();
    render(
      <MeshyPresetFields
        preset={{ ...base, aiModel: 'meshy-t2.5' }}
        onChange={() => {}}
        variant="prop"
      />
    );
    expect(screen.getByRole('switch', { name: 'Remesh' })).toBeDisabled();
    expect(screen.getByRole('combobox', { name: 'Topology' })).toBeDisabled();
    expect(screen.getByRole('textbox', { name: 'Faces' })).toBeInTheDocument();
  });

  it('show the character height for characters', async () => {
    await openMockWorkspace();
    render(<MeshyPresetFields preset={base} onChange={() => {}} variant="character" />);
    expect(screen.getByText('Meshy adds a walk and a run')).toBeInTheDocument();
  });
});
