import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { ModelFile } from '@/ipc/client';
import { ArtifactsList } from '.';
import { artifactRows } from './artifactRows';

afterEach(cleanup);

const file = (model: string, rawBytes: number | null, shippedBytes: number | null): ModelFile => ({
  model,
  assetName: null,
  version: 1,
  rawBytes,
  shippedBytes,
});

describe('the generated artifacts list', () => {
  it('says raw to compressed per file and what is still being written', () => {
    const rows = artifactRows([file('b_v1', null, null), file('a_v1', 4.6e6, 1.1e6)], true);
    expect(rows.map(r => r.file)).toEqual(['a_v1.glb', 'b_v1.glb']);
    expect(rows[0]!.size).toBe('4.6 MB → 1.1 MB');
    expect(rows[1]!.size).toBe('exporting...');
    expect(artifactRows([file('lod', null, 0.41e6)], false)[0]!.size).toBe('new · 410 KB');
  });

  it('folds long lists and names a game project list files', async () => {
    const many = Array.from({ length: 6 }, (_, n) => file(`m${n}_v1`, 2e6, 1e6));
    render(<ArtifactsList models={many} busy={false} game />);
    expect(screen.getByRole('region', { name: 'Generated files' })).toBeVisible();
    expect(screen.queryByText('m5_v1.glb')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Show all 6' }));
    expect(screen.getByText('m5_v1.glb')).toBeVisible();
  });

  it('shows nothing before a model exists', () => {
    const { container } = render(<ArtifactsList models={[]} busy={false} />);
    expect(container).toBeEmptyDOMElement();
  });
});
