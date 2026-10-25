import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands, type InspectView } from '@/ipc/client';
import { stopsOf } from '@/state/inspector';
import { useInspectorStore } from '@/state/use-inspector-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import CameraStops from './CameraStops';

let view: InspectView;

beforeEach(async () => {
  await openMockWorkspace();
  view = await commands.inspectProject('moon-island');
});
afterEach(() => {
  clearMocks();
  useInspectorStore.setState({ stop: null });
});

describe('the camera stops', () => {
  it('shows each stop with its live render, or a camera while there is none', () => {
    const { container } = render(
      <CameraStops view={view} stops={stopsOf(view)} shots={{ 1: 'data:image/jpeg;sakura' }} />
    );
    expect(screen.getByRole('heading', { name: 'Camera stops' })).toBeInTheDocument();
    expect(screen.getByText(/click to fly/)).toBeInTheDocument();
    const images = container.querySelectorAll('img');
    expect([...images].map(i => i.getAttribute('src'))).toEqual(['data:image/jpeg;sakura']);
    expect(screen.getAllByRole('button')).toHaveLength(5);
  });

  it('flies to the stop clicked and rings it', async () => {
    render(<CameraStops view={view} stops={stopsOf(view)} shots={{}} />);
    await userEvent.click(screen.getByRole('button', { name: 'Fly to Canopy' }));
    expect(useInspectorStore.getState().stop?.index).toBe(3);
    expect(screen.getByRole('button', { name: 'Fly to Canopy' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
  });
});
