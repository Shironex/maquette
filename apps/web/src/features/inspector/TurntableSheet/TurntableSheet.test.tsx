import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { RENDER_DEFAULTS } from '@/state/renders';
import { useRenderTabStore } from '@/state/use-render-tab-store';
import { clearMocks } from '@/test/mock-ipc';
import { openMockProject } from '@/test/openMockProject';
import TurntableSheet from './TurntableSheet';

const source = { from: 'project', project: 'frostveil' } as const;
afterEach(() => {
  clearMocks();
  useRenderTabStore.setState({ turntableOpen: false, settings: RENDER_DEFAULTS, preview: null });
});

describe('the turntable sheet', () => {
  it('holds the fixed GIF rules, the moving area and the size against 2 MB', async () => {
    await openMockProject();
    useRenderTabStore.setState({ style: 'turntable', turntableOpen: true });
    render(<TurntableSheet source={source} assetId="crystal-lantern" name="Crystal lantern" />);
    expect(screen.queryByText('One palette for the whole loop.')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Animated GIF' }));
    expect(screen.getByText('One palette for the whole loop.')).toBeVisible();
    expect(screen.getByText('20 fps')).toBeVisible();
    await waitFor(() => expect(screen.getByLabelText('Moving area')).toBeVisible(), {
      timeout: 3000,
    });
    expect(screen.getByRole('meter', { name: /2 MB target/ })).toBeVisible();
    expect(screen.getByText(/\/ 2 MB target/)).not.toHaveTextContent('...');
  });
});
