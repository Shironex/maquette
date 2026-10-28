import { waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { events } from '@/ipc/generated/bindings';
import { clearMocks, openMockRefs } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useRefsStore } from './use-refs-store';

afterEach(() => {
  clearMocks();
  useRefsStore.setState({ sheet: null, view: null, error: null, busy: false });
});

const FRONT = 'refs/guardian_statues_front.png';
const CROP = 'refs/guardian_statues_crop.png';

const boardRefs = () =>
  useAppStore.getState().workspace?.project?.assets?.find(a => a.id === 'guardian-statues')?.refs;

async function openGuardian() {
  const mock = await openMockRefs();
  await events.conceptUpdate.listen(e => useRefsStore.getState().onConcept(e.payload));
  await useRefsStore.getState().open('frostveil', 'guardian-statues');
  return mock;
}

describe('the refs store', () => {
  it('reads the tagged refs of an asset', async () => {
    await openGuardian();
    const view = useRefsStore.getState().view!;
    expect(view.tagged).toBe(true);
    expect(view.refs.find(r => r.view === 'front')?.file).toBe(FRONT);
    expect(view.refs.filter(r => r.view === 'other')).toHaveLength(2);
  });

  it('saves a tag at once, swapping with the ref that held the view', async () => {
    const { sent } = await openGuardian();
    await useRefsStore.getState().tag(CROP, 'front');
    const refs = useRefsStore.getState().view!.refs;
    expect(refs.find(r => r.file === CROP)?.view).toBe('front');
    expect(refs.find(r => r.file === FRONT)?.view).toBe('other');
    expect(sent('refs_set_views')).toHaveLength(1);
    expect(boardRefs()?.find(r => r.file === CROP)?.view).toBe('front');
  });

  it('draws the missing views and puts a candidate in its slot', async () => {
    const { sent } = await openGuardian();
    await useRefsStore.getState().makeViews(['left', 'back', 'right'], 1, true);
    expect(sent('refs_make_views')[0]).toMatchObject({
      request: { asset: 'guardian-statues', views: ['left', 'back', 'right'], count: 1 },
    });
    await waitFor(() =>
      expect(useRefsStore.getState().view!.directions[0]?.images).toHaveLength(3)
    );
    const left = useRefsStore.getState().view!.directions[0]!.images.find(i => i.title === 'left')!;
    await useRefsStore.getState().use(left.file, 'left');
    const refs = useRefsStore.getState().view!.refs;
    expect(refs.find(r => r.view === 'left')?.file).toBe('refs/guardian_statues_left.png');
  });

  it('cleans a ref: the copy takes its place and view, the original leaves', async () => {
    await openGuardian();
    await useRefsStore.getState().clean(CROP, 'studio-grey', false, false);
    await waitFor(() =>
      expect(useRefsStore.getState().view!.directions[0]?.images).toHaveLength(1)
    );
    const copy = useRefsStore.getState().view!.directions[0]!.images[0]!;
    expect(copy.title).toBe(CROP);
    await useRefsStore.getState().use(copy.file);
    const refs = useRefsStore.getState().view!.refs;
    expect(refs.some(r => r.file === CROP)).toBe(false);
    expect(refs.find(r => r.file === 'refs/guardian_statues_crop_clean.png')?.view).toBe('other');
  });
});
