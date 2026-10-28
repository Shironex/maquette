import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import LayoutPanel from './LayoutPanel';

beforeEach(async () => {
  await openMockWorkspace();
  await useLayoutStore.getState().load('frostveil');
});
afterEach(() => {
  clearMocks();
  useLayoutStore.setState({
    view: null,
    project: null,
    selection: null,
    preview: null,
    pose: null,
  });
});

function panel() {
  render(<LayoutPanel view={useLayoutStore.getState().view!} dirty={false} />);
  return screen.getByRole('complementary', { name: 'Layout details' });
}

describe('the layout panel', () => {
  it('lists the models, marking those in the first view, and the stops', () => {
    const aside = panel();
    expect(
      within(aside).getByRole('button', { name: /^Veil platform, in the first view/ })
    ).toBeInTheDocument();
    const stops = within(aside)
      .getAllByRole('listitem')
      .filter(li => li.closest('ol'));
    expect(stops).toHaveLength(5);
    expect(within(stops[0]!).getByText('Arrival')).toBeInTheDocument();
    expect(within(stops[0]!).getByText('36°')).toBeInTheDocument();
    expect(within(aside).getByText('5 of 24')).toBeInTheDocument();
  });

  it('selects a model and a stop from the lists', async () => {
    const aside = panel();
    await userEvent.click(
      within(aside).getByRole('button', { name: /^Crystal lantern, in the first view/ })
    );
    expect(useLayoutStore.getState().selection).toEqual({ kind: 'model', id: 'crystal-lantern' });
    expect(within(aside).getByRole('heading', { name: 'Selected model' })).toBeInTheDocument();
    await userEvent.click(within(aside).getByText('Veil gate'));
    expect(useLayoutStore.getState().selection).toEqual({ kind: 'stop', id: 'veil-gate' });
    expect(within(aside).getByRole('heading', { name: 'Selected stop' })).toBeInTheDocument();
  });

  it('looks through a stop from its row', async () => {
    const aside = panel();
    await userEvent.click(within(aside).getByRole('button', { name: 'Look through Keystone' }));
    expect(useLayoutStore.getState().preview?.id).toBe('keystone');
    expect(within(aside).getByRole('button', { name: 'Stop looking' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
  });

  it('puts a desk under the models and sizes it', async () => {
    const aside = panel();
    await userEvent.click(within(aside).getByRole('switch', { name: /A desk under the models/ }));
    expect(useLayoutStore.getState().draft.desk).not.toBeNull();
    const width = within(aside).getByLabelText('Width');
    await userEvent.clear(width);
    await userEvent.type(width, '2.5{Enter}');
    expect(useLayoutStore.getState().draft.desk?.[0]).toBe(2.5);
  });
});
