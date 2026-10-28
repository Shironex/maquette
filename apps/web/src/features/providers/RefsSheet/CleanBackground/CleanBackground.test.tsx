import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { events } from '@/ipc/generated/bindings';
import { useRefsStore } from '@/state/use-refs-store';
import { clearMocks, openMockRefs } from '@/test/mock-ipc';
import { RefsSheet } from '..';

const CROP = 'refs/guardian_statues_crop.png';

afterEach(() => {
  clearMocks();
  useRefsStore.setState({ sheet: null, view: null, error: null, busy: false });
});

describe('Clean background', () => {
  it('draws a clean copy, then puts it in the place of the original', async () => {
    const { sent } = await openMockRefs();
    await events.conceptUpdate.listen(e => useRefsStore.getState().onConcept(e.payload));
    render(<RefsSheet />);
    await useRefsStore.getState().open('frostveil', 'guardian-statues', 'clean', CROP);
    const sheet = await screen.findByRole('dialog', { name: 'Clean background, Guardian statues' });
    await userEvent.click(within(sheet).getByRole('button', { name: 'Transparent' }));
    await userEvent.click(within(sheet).getByRole('button', { name: 'Clean background' }));
    expect(sent('refs_clean')[0]).toMatchObject({
      request: { file: CROP, ground: 'transparent', pad: false, extend: false },
    });
    await userEvent.click(await within(sheet).findByRole('button', { name: 'Use the clean copy' }));
    expect(useRefsStore.getState().sheet?.mode).toBe('refs');
    expect(useRefsStore.getState().view!.refs.some(r => r.file === CROP)).toBe(false);
  });

  it('keeps the original', async () => {
    await openMockRefs();
    render(<RefsSheet />);
    await useRefsStore.getState().open('frostveil', 'guardian-statues', 'clean', CROP);
    await userEvent.click(await screen.findByRole('button', { name: 'Keep the original' }));
    expect(useRefsStore.getState().sheet?.mode).toBe('refs');
  });
});
