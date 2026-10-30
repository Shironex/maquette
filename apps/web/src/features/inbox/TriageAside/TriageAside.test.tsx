import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { InboxItem } from '@/ipc/client';
import type { AcceptPlan } from '@/state/triage';
import { useInboxStore } from '@/state/use-inbox-store';
import { useTriageStore } from '@/state/use-triage-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { TriageAside } from '.';
import { parseTags } from './TriageAside.hooks';

let replaced: InboxItem[] = [];
const plan: AcceptPlan = {
  ids: ['a.glb', 'b.glb', 'c.glb'],
  fresh: 2,
  later: ['Observatory arch v4'],
  handoffs: 1,
  skipped: 1,
};

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'triage');
  await useInboxStore.getState().load();
  await useTriageStore.getState().loadDiscarded();
  replaced = useInboxStore.getState().snapshot!.items.filter(i => i.replacedBy != null);
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
  useTriageStore.setState({ discarded: [] });
});

describe('the triage side column', () => {
  it('says what accepting the selection does, and accepts it with the library after', async () => {
    const onAccept = vi.fn();
    render(
      <TriageAside
        count={4}
        plan={plan}
        replaced={[]}
        working={false}
        onAccept={onAccept}
        onDiscard={() => {}}
      />
    );
    const aside = screen.getByRole('complementary', { name: 'What the selection does' });
    expect(aside).toHaveTextContent('3 become versions');
    expect(aside).toHaveTextContent(
      '2 as v1 of a new model, 1 as the next version (Observatory arch v4)'
    );
    expect(aside).toHaveTextContent('1 By hand send closes');
    expect(aside).toHaveTextContent('1 stay here');
    await userEvent.click(screen.getByRole('checkbox', { name: 'Add to library after' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Tag all with' }), 'Village, props');
    await userEvent.click(screen.getByRole('button', { name: 'Accept 3 and add to library' }));
    expect(onAccept).toHaveBeenCalledWith({ library: true, tags: ['village', 'props'] });
  });

  it('offers to discard the models a redo replaced, and lists the discarded ones', async () => {
    const onDiscard = vi.fn();
    render(
      <TriageAside
        count={0}
        plan={{ ids: [], fresh: 0, later: [], handoffs: 0, skipped: 0 }}
        replaced={replaced}
        working={false}
        onAccept={() => {}}
        onDiscard={onDiscard}
      />
    );
    expect(screen.getByText(/Tick models, or a whole group/)).toBeInTheDocument();
    expect(screen.getByText('v2 is newer · 3.4 MB')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Discard 2 replaced' }));
    expect(onDiscard).toHaveBeenCalledWith(['crystal_lantern_redo.glb', 'observatory_arch_v3.glb']);
    expect(screen.getByRole('region', { name: 'Recently discarded' })).toBeInTheDocument();
  });

  it('reads tags lowercased, without blanks or repeats', () => {
    expect(parseTags(' Village, props,,village ')).toEqual(['village', 'props']);
  });
});
