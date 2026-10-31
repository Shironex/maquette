import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useRenderStore } from '@/state/use-render-store';
import { clearMocks } from '@/test/mock-ipc';
import { openMockProject } from '@/test/openMockProject';
import RenderSheet from './RenderSheet';

const source = { from: 'project', project: 'frostveil' } as const;

beforeEach(async () => {
  await openMockProject();
  useRenderStore.setState({ recent: [], live: null, record: null, open: false });
});
afterEach(() => clearMocks());

const open = async (keys: string[]) => {
  await useRenderStore.getState().show(source, keys, 'Opened from the Board selection');
  render(<RenderSheet />);
  return screen.findByRole('dialog', { name: /^Render/ });
};

describe('the batch render sheet', () => {
  it('lists the selection and adds a style to all', async () => {
    const sheet = await open(['crystal-lantern', 'elsheria']);
    expect(within(sheet).getByText('Crystal lantern')).toBeVisible();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Blueprint' }));
    expect(useRenderStore.getState().rows['elsheria']).toEqual(['clay', 'blueprint']);
    expect(within(sheet).getByRole('button', { name: 'Render 4 files' })).toBeEnabled();
  });

  it('warns that toon is left out for a model with no texture', async () => {
    const sheet = await open(['observatory-arch']);
    await userEvent.click(within(sheet).getByRole('button', { name: 'Toon' }));
    expect(within(sheet).getByText(/toon would come out flat grey/)).toBeVisible();
  });

  it('runs one file at a time, and keeps count', async () => {
    const sheet = await open(['crystal-lantern', 'elsheria']);
    await userEvent.click(within(sheet).getByRole('button', { name: 'Render 2 files' }));
    const record = useRenderStore.getState().record;
    expect(record?.state).toBe('running');
    await waitFor(async () => {
      const now = await commands.renderGet(record!.id);
      expect(now.state).toBe('done');
    });
  });
});
