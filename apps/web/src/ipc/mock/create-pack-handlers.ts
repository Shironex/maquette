/**
 * Export packs in mock mode, as the core runs them: a plan of a source, the
 * presets, a pack that runs one model at a time on a timer and tells the
 * web view after each step, decisions on what the check flagged, the zip,
 * stop and go on. Nothing is written; the records live in memory.
 */
import type {
  KindRow,
  LibraryView,
  PackDecision,
  PackItem,
  PackPlan,
  PackPreset,
  PackRecord,
  PackRequest,
  PackSettings,
  PackSource,
  ProjectManifest,
  ZipChoice,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import {
  BUILT_IN,
  budgetOf,
  modelsOfItems,
  modelsOfProject,
  newItem,
  outcome,
  seedRecords,
} from './pack-fixtures';

interface PackContext {
  library: () => LibraryView;
  project: (id: string) => ProjectManifest;
  root: () => string;
  emit: (record: PackRecord) => void;
  /** Milliseconds per step of a model's run. */
  pace: number;
  /** The library state the mock opened with; a library at scale seeds two packs. */
  state: string | null;
}

const KIND_LABEL = { prop: 'Props', character: 'Characters', scene: 'Scenes' } as const;
const clone = (r: PackRecord): PackRecord => structuredClone(r);

export function createPackHandlers(ctx: PackContext) {
  const records = new Map<string, PackRecord>();
  const attempts = new Map<string, number>();
  const stopping = new Set<string>();
  let saved: PackPreset[] = [];
  let seeded = false;

  const seed = () => {
    if (seeded) return;
    seeded = true;
    if (ctx.state !== 'scale') return;
    for (const r of seedRecords(ctx.library().items, ctx.root(), clock.nowMs())) {
      records.set(r.id, r);
    }
  };
  const record = (id: string) => {
    seed();
    return records.get(id) ?? fail('not-found', `Could not find the pack ${id}.`);
  };
  const tell = (r: PackRecord) => {
    records.set(r.id, r);
    ctx.emit(clone(r));
    return clone(r);
  };
  const presets = () => [BUILT_IN, ...[...saved].sort((a, b) => a.name.localeCompare(b.name))];

  function models(source: PackSource) {
    const view = ctx.library();
    if (source.from === 'project') {
      const p = ctx.project(source.project);
      return { label: `of ${p.name}`, list: modelsOfProject(p, `${ctx.root()}/projects/${p.id}`) };
    }
    const ids =
      source.from === 'entries'
        ? source.entries
        : (view.collections.find(c => c.id === source.collection)?.entries ??
          fail('not-found', `Could not find the collection ${source.collection}.`));
    const items = view.items.filter(i => ids.includes(i.entry.id));
    return { label: 'from the library', list: modelsOfItems(items) };
  }

  function plan(source: PackSource, settings: PackSettings): PackPlan {
    const { label, list } = models(source);
    if (!list.length) fail('invalid', 'There is no model to pack.');
    const kinds: KindRow[] = (['prop', 'character', 'scene'] as const)
      .map(kind => {
        const of = list.filter(m => m.kind === kind);
        return {
          kind,
          label: KIND_LABEL[kind],
          count: of.length,
          biggest: Math.max(0, ...of.map(m => m.stats?.triangles ?? 0)),
          budget: budgetOf(settings, kind),
        };
      })
      .filter(k => k.count > 0);
    const tris = list.reduce(
      (n, m) => n + Math.min(m.stats?.triangles ?? 0, budgetOf(settings, m.kind) ?? Infinity),
      0
    );
    const perTri = settings.format === 'glb' ? 30 : settings.format === 'both' ? 70 : 40;
    const texture = settings.texture ? settings.texture * settings.texture * 0.9 : 3_000_000;
    return {
      label: `${list.length} ${list.length === 1 ? 'model' : 'models'} ${label}`,
      models: list,
      kinds,
      estimateBytes: tris * perTri + list.length * texture * 0.35,
      mixamo: list.filter(m => m.mixamo).length,
      skipped: [],
    };
  }

  function settle(r: PackRecord) {
    if (r.items.every(i => i.state === 'pass' || i.state === 'kept')) return zipNow(r, 'passing');
    tell({ ...r, state: 'waiting' });
  }

  function zipNow(r: PackRecord, choice: ZipChoice) {
    const keeps = (i: PackItem) =>
      i.state === 'pass' || i.state === 'kept' || (choice === 'all' && i.state === 'warn');
    const left = r.items.filter(i => !keeps(i)).length;
    tell({ ...r, state: 'zipping' });
    setTimeout(() => {
      const bytes = r.items.filter(keeps).reduce((n, i) => n + (i.bytes ?? 0), 0);
      tell({
        ...record(r.id),
        state: 'done',
        zip: r.settings.zip ? `${r.dir}.zip` : null,
        finishedAt: clock.now().toISOString(),
        bytes,
        message: left
          ? `${left} ${left === 1 ? 'model' : 'models'} left out of the pack.`
          : r.message,
      });
    }, ctx.pace * 2);
  }

  /** Runs the next queued model, or settles the pack. */
  function step(id: string) {
    const r = record(id);
    if (r.state !== 'running') return;
    const index = r.items.findIndex(i => i.state === 'queued');
    if (index < 0 || stopping.has(id)) {
      const asked = stopping.delete(id);
      if (asked && index >= 0) {
        const done = r.items.filter(i => i.state !== 'queued').length;
        const message = `Stopped with ${done} of ${r.items.length} done, model ${index + 1} is next`;
        tell({ ...r, state: 'stopped', message });
        return;
      }
      settle(r);
      return;
    }
    const running = r.items.map((i, n) => (n === index ? { ...i, state: 'running' as const } : i));
    tell({ ...r, items: running });
    setTimeout(() => {
      const now = record(id);
      const item = now.items[index]!;
      const key = `${id}/${item.model.key}`;
      const tries = attempts.get(key) ?? 0;
      attempts.set(key, tries + 1);
      const done = outcome(item, index, tries, now.settings, 14_000 + (index % 5) * 1_300);
      tell({ ...now, items: now.items.map((i, n) => (n === index ? done : i)) });
      step(id);
    }, ctx.pace * 3);
  }

  function launch(r: PackRecord) {
    tell({ ...r, state: 'running', message: null });
    setTimeout(() => step(r.id), ctx.pace);
    return clone(records.get(r.id)!);
  }

  function start({ request }: Args) {
    seed();
    const req = request as PackRequest;
    const settings = req.settings;
    if (!/[a-z0-9]/i.test(settings.name)) fail('invalid', 'Give the pack a name.');
    const planned = plan(req.source, settings);
    const stamp = clock.now().toISOString().replace(/\D/g, '').slice(0, 14);
    const parent = req.out?.trim() || `${ctx.root()}/exports`;
    for (const [id, old] of records)
      if (old.name === settings.name && old.dir === `${parent}/${settings.name}`)
        records.delete(id);
    const r: PackRecord = {
      version: 1,
      id: `${settings.name}-${stamp}`,
      name: settings.name,
      label: planned.label,
      source: req.source,
      settings,
      dir: `${parent}/${settings.name}`,
      zip: null,
      state: 'running',
      items: planned.models.map(m => newItem(m, budgetOf(settings, m.kind))),
      createdAt: clock.now().toISOString(),
      finishedAt: null,
      bytes: null,
      message: null,
      preset: req.preset,
    };
    saved = saved.map(p => (p.id === req.preset ? { ...p, used: p.used + 1 } : p));
    return launch(r);
  }

  function decide({ id, decision }: Args) {
    const r = record(id as string);
    const d = decision as PackDecision;
    const at = r.items.findIndex(i => i.model.key === d.key);
    if (at < 0) fail('not-found', `The pack has no model ${d.key}.`);
    const item = r.items[at]!;
    if (item.state !== 'warn' && item.state !== 'fail') {
      fail('invalid', `${item.model.name} has nothing to decide.`);
    }
    if (d.choice === 'raise' && d.triangles <= (item.budget ?? 0)) {
      fail('invalid', 'Raise the budget above what it is now.');
    }
    const next: PackItem =
      d.choice === 'keep'
        ? { ...item, state: 'kept' }
        : {
            ...item,
            state: 'queued',
            note: null,
            budget: d.choice === 'raise' ? d.triangles : item.budget,
          };
    const changed = { ...r, items: r.items.map((i, n) => (n === at ? next : i)) };
    if (r.state === 'running') return tell(changed);
    if (next.state === 'queued') return launch(changed);
    settle(changed);
    return clone(records.get(r.id)!);
  }

  return {
    handlers: {
      pack_plan: ({ source, settings }) => plan(source as PackSource, settings as PackSettings),
      pack_presets: () => presets(),
      pack_save_preset: ({ name, settings }) => {
        const label = String(name).trim();
        if (!label) fail('invalid', 'Give the preset a name.');
        if (label.toLowerCase() === BUILT_IN.name.toLowerCase()) {
          fail('invalid', "That name is the built in preset's. Pick another.");
        }
        const id = label.toLowerCase().replace(/[^a-z0-9]+/g, '-');
        const preset: PackPreset = {
          id,
          name: label,
          settings: { ...(settings as PackSettings), name: '' },
          builtIn: false,
          savedAt: clock.now().toISOString(),
          used: 0,
        };
        saved = [...saved.filter(p => p.id !== id), preset];
        return presets();
      },
      pack_start: start,
      pack_get: ({ id }) => clone(record(id as string)),
      pack_recent: () => {
        seed();
        return [...records.values()]
          .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
          .slice(0, 8)
          .map(clone);
      },
      pack_decide: decide,
      pack_zip: ({ id, choice }) => {
        const r = record(id as string);
        if (r.state !== 'waiting') fail('invalid', 'The pack is not waiting for the zip.');
        zipNow(r, choice as ZipChoice);
        return clone(records.get(r.id)!);
      },
      pack_stop: ({ id }) => {
        const r = record(id as string);
        if (r.state !== 'running') fail('invalid', 'The pack is not running.');
        stopping.add(r.id);
        return clone(r);
      },
      pack_resume: ({ id }) => {
        const r = record(id as string);
        if (r.state !== 'stopped') fail('invalid', 'Only a stopped pack goes on.');
        return launch(r);
      },
      pack_reveal: ({ id }) => {
        record(id as string);
        return null;
      },
    } as Record<string, (args: Args) => unknown>,
  };
}
