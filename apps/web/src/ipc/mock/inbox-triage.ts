/**
 * The inbox commands' triage side, as the core does it: where each model
 * lands (`nextVersion`, `replacedBy`) and the discarded models with Restore.
 */
import type { DiscardedModel, InboxItem, ProjectManifest } from '../client';
import { clock } from '@/lib/clock';
import { fail } from './backend-options';
import { seedDiscarded, type DiscardedSeed } from './triage-fixtures';

/** Fills each item's next version and the newer version that replaced it, as `inbox::place` does. */
export function placeItems(
  items: InboxItem[],
  project: (id: string) => ProjectManifest | null
): InboxItem[] {
  return items.map(item => {
    const goal = item.link ?? item.aim;
    if (!goal) return { ...item, nextVersion: null, replacedBy: null };
    const p = project(goal.project);
    if (!p) return { ...item, nextVersion: null, replacedBy: null };
    const asset = (p.assets ?? []).find(a => a.id === goal.asset);
    if (!asset) return { ...item, nextVersion: 1, replacedBy: null };
    const newest = Math.max(0, ...asset.versions.map(v => v.version));
    const later = asset.versions
      .filter(v => Date.parse(v.createdAt) > (item.modifiedMs ?? 0))
      .map(v => v.version);
    return {
      ...item,
      nextVersion: newest + 1,
      replacedBy: later.length ? Math.max(...later) : null,
    };
  });
}

/** The models in `inbox/.discarded`, newest first, and the item each puts back. */
export function createDiscarded(state: string | null, dir: string) {
  let kept: DiscardedSeed[] = seedDiscarded(state, dir, clock.nowMs());
  return {
    list: (): DiscardedModel[] => kept.map(k => k.model),
    add: (item: InboxItem) => {
      const taken = new Set(kept.map(k => k.model.name));
      const stem = item.id.replace(/\.[^.]+$/, '');
      const ext = item.id.slice(stem.length);
      let name = item.id;
      for (let n = 2; taken.has(name); n += 1) name = `${stem}-${n}${ext}`;
      const model: DiscardedModel = {
        name,
        id: item.id,
        file: `${dir}/.discarded/${name}`,
        bytes: item.bytes,
        discardedMs: clock.nowMs(),
        assetName: (item.link ?? item.aim)?.assetName ?? null,
        replacedBy: item.replacedBy ?? null,
      };
      kept = [{ model, item }, ...kept];
      return model;
    },
    /** Takes `name` out of the folder and answers the item to put back under a free inbox name. */
    restore: (name: string, inbox: readonly InboxItem[]): InboxItem => {
      const found = kept.find(k => k.model.name === name);
      if (!found) fail('not-found', `${name} among the discarded models`);
      kept = kept.filter(k => k !== found);
      const taken = new Set(inbox.map(i => i.id));
      const wanted = found.model.id;
      const stem = wanted.replace(/\.[^.]+$/, '');
      const ext = wanted.slice(stem.length);
      let id = wanted;
      for (let n = 2; taken.has(id); n += 1) id = `${stem}-${n}${ext}`;
      // The send its discard closed stays closed; the model is aimed at its asset instead.
      const { link, aim } = found.item;
      const back = link
        ? { project: link.project, asset: link.asset, assetName: link.assetName }
        : aim;
      return { ...found.item, id, file: `${dir}/${id}`, link: null, aim: back };
    },
  };
}
