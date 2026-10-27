import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type InspectView } from '@/ipc/client';
import { useInspectorStore } from '@/state/use-inspector-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import type { CompareCanvasProps } from '../CompareCanvas';
import Inspector from './Inspector';

const canvas = vi.hoisted(() => ({ props: null as CompareCanvasProps | null }));
vi.mock('@/features/inspector/CompareCanvas', () => ({
  default: (props: CompareCanvasProps) => {
    canvas.props = props;
    return null;
  },
}));

let view: InspectView;

beforeEach(async () => {
  await openMockWorkspace();
  view = await commands.inspectProject('frostveil');
});
afterEach(() => {
  clearMocks();
  canvas.props = null;
  useInspectorStore.setState({ stop: null, selected: null, mode: 'shaded', samples: [] });
});

describe('the Inspector', () => {
  it('compares the scene versions and starts at the first camera stop', async () => {
    render(<Inspector view={view} />);
    expect(screen.getByRole('heading', { name: 'Scene comparison' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /v6\s*vs\s*v7/ })).toBeInTheDocument();
    expect(useInspectorStore.getState().stop?.index).toBe(0);
    expect(await screen.findByRole('button', { name: 'Fly to Arrival' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await waitFor(() =>
      expect(canvas.props).toMatchObject({ kind: 'scene', mode: 'shaded', turntable: false })
    );
  });

  it('turns the view and switches its look', async () => {
    render(<Inspector view={view} />);
    await screen.findByLabelText('Compare position');
    await userEvent.click(screen.getByRole('button', { name: 'Rotate the view' }));
    expect(screen.getByRole('button', { name: 'Stop rotating' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(canvas.props?.turntable).toBe(true);
    await userEvent.click(screen.getByRole('button', { name: 'Wireframe' }));
    expect(useInspectorStore.getState().mode).toBe('wireframe');
  });

  it("shows the right side's character first, and another model from its budget row", async () => {
    render(<Inspector view={view} />);
    const panel = screen.getByRole('complementary', { name: 'Model details' });
    expect(within(panel).getByText('Elsheria', { selector: 'span.truncate' })).toBeInTheDocument();
    await userEvent.click(within(panel).getByRole('button', { name: /^Veil gate/ }));
    expect(useInspectorStore.getState().selected).toBe('shipped/veil_gate');
    expect(within(panel).getByText('Veil gate', { selector: 'span.truncate' })).toBeInTheDocument();
  });

  it('cannot run the pipeline before it is read', () => {
    render(<Inspector view={view} />);
    expect(screen.getByRole('button', { name: 'Run pipeline' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });
});
