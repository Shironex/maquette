/**
 * The grip library in mock mode: library grips kept in memory with their
 * revisions, with the core's checks. A save always adds a revision and
 * never overwrites one; a save or a removal against a grip that moved since
 * it was read is refused (`moved`); a pose that holds what the newest
 * revision holds has nothing to save; the only revision left cannot be
 * removed on its own. A save from a pose key change ends the approval an
 * agent raised for it. `?library=demo` opens with six grips (see
 * grip-library-fixtures.ts).
 */
import type {
  GripFile,
  GripSave,
  GripSavePlan,
  GripSaved,
  LibraryGripDoc,
  LibraryGripEntry,
  LibraryGripSummary,
} from '../client';
import { events } from '../generated/bindings';
import { clock } from '@/lib/clock';
import {
  GRIP_VERSION,
  WEAPONS,
  handsWords,
  isGripBone,
  otherHand,
  sidesOf,
  weaponGroup,
} from '@/state/recipe-grips';
import { fail, type Args } from './backend-options';
import { demoGrips, gripRevision, newGripUid, type StoredGrip } from './grip-library-fixtures';
import { currentGripSaveAsks } from './grip-save-asks';
import { slugOf } from './plan-store';
import { uniqueName, versionOf } from './recipe-files';

const changed = () => void events.gripLibraryChanged.emit().catch(() => {});

const plural = (n: number) => (n === 1 ? '1 bone' : `${n} bones`);
const bonesOf = (grip: GripFile) => Object.keys(grip.pose).length;
const sameHands = (a: GripFile, b: GripFile) =>
  a.hands.holds === b.hands.holds && (a.hands.supports ?? null) === (b.hands.supports ?? null);

/** The core's `changes`: what `after` changes against `before`, a line each. */
export function gripChanges(before: GripFile | null, after: GripFile): string[] {
  if (!before) return [`A new grip for ${handsWords(after)}: ${plural(bonesOf(after))}.`];
  const out: string[] = [];
  if (before.weapon !== after.weapon)
    out.push(`Holds ${after.weapon} instead of ${before.weapon}.`);
  if (!sameHands(before, after))
    out.push(`Poses ${handsWords(after)}, where it posed ${handsWords(before)}.`);
  const added = Object.keys(after.pose).filter(b => !(b in before.pose)).length;
  const removed = Object.keys(before.pose).filter(b => !(b in after.pose)).length;
  const turned = Object.entries(after.pose).filter(([bone, turn]) => {
    const old = before.pose[bone];
    return old && old.some((v, i) => v !== turn[i]);
  }).length;
  if (added) out.push(`Poses ${plural(added)} more.`);
  if (removed) out.push(`Leaves ${plural(removed)} out.`);
  if (turned) out.push(`Turns ${plural(turned)} another way.`);
  return out;
}

/** The core's `same_pose`: the same weapon, hands and turns; the name does not count. */
const samePose = (a: GripFile, b: GripFile) => {
  const bones = Object.keys(a.pose);
  return (
    a.weapon === b.weapon &&
    sameHands(a, b) &&
    bones.length === Object.keys(b.pose).length &&
    bones.every(bone => b.pose[bone]?.every((v, i) => v === a.pose[bone]![i]))
  );
};

/** The most bones a grip poses, and the furthest it turns one each way, as the core has them. */
const MAX_GRIP_BONES = 64;
const MAX_TURN_DEGREES = 180;

/** The core's file checks (`recipes/grips.rs`), in its order and its words. */
function check(grip: GripFile) {
  const name = grip.name;
  if (!name) fail('invalid', 'Give the grip a name.');
  if (name.length > 120) fail('invalid', 'Keep the grip under 120 characters.');
  const bones = Object.keys(grip.pose);
  if (!bones.length) fail('invalid', `The grip ${name} turns no bone. Turn one, or take it out.`);
  if (bones.length > MAX_GRIP_BONES)
    fail('invalid', `The grip ${name} turns at most ${MAX_GRIP_BONES} bones.`);
  for (const [bone, turn] of Object.entries(grip.pose))
    if (!turn.every(v => typeof v === 'number' && Math.abs(v) <= MAX_TURN_DEGREES))
      fail(
        'invalid',
        `The grip ${name} turns ${bone} more than ${MAX_TURN_DEGREES} degrees each way.`
      );
  const stray = bones.find(b => !isGripBone(b));
  if (stray)
    fail('invalid', `A grip poses a hand, its fingers and the arm above it, not ${stray}.`);
  const sides = sidesOf(grip.pose);
  const { holds, supports } = grip.hands;
  if (!sides[holds])
    fail(
      'invalid',
      `The grip ${name} says the ${holds} hand holds, but its pose turns no bone on that side.`
    );
  if (supports && (supports === holds || !sides[supports]))
    fail(
      'invalid',
      `The grip ${name} says the ${supports} hand supports, but its pose turns no bone on that side.`
    );
  if (!supports && sides[otherHand(holds)])
    fail('invalid', `The grip ${name} poses both hands, so say which holds and which supports.`);
}

/** The core's `check_weapon`: a save holds one of the weapons the library sorts by. */
function checkWeapon(weapon: string) {
  if (WEAPONS.some(([id]) => id === weapon)) return;
  fail(
    'invalid',
    `"${weapon}" is not a weapon a grip holds. Use one of ${WEAPONS.map(([id]) => id).join(', ')}.`
  );
}

export function createGripLibraryHandlers(ctx: { state: string | null }) {
  let library: Map<string, StoredGrip> | null = null;
  const entries = () => {
    if (!library) library = ctx.state === 'demo' ? demoGrips() : new Map();
    return library;
  };
  const entryOf = (id: unknown) => {
    const found = entries().get(String(id));
    if (!found) fail('not-found', `Not found: the library grip ${String(id)}`);
    return found;
  };
  const latestOf = (e: StoredGrip) => Math.max(0, ...e.entry.revisions.map(r => r.revision));
  const fileAt = (e: StoredGrip, revision: number | null) => {
    const at = revision ?? latestOf(e);
    const file = e.files.get(at);
    if (!file || !e.entry.revisions.some(r => r.revision === at))
      fail('not-found', `Not found: revision r${at} of the library grip ${e.entry.id}`);
    return file;
  };
  const checkExpected = (e: StoredGrip, expected: unknown) => {
    const now = versionOf(e.entry);
    if (expected != null && expected !== now)
      fail(
        'moved',
        `The library grip ${e.entry.id} changed since it was read (its version is now ${now}). Read it again.`
      );
  };
  const freeId = (name: string) => {
    const base = /[a-z0-9]/i.test(name) ? slugOf(name) : 'grip';
    let id = base;
    for (let n = 2; entries().has(id); n += 1) id = `${base}-${n}`;
    return id;
  };
  const names = () => [...entries().values()].map(e => e.entry.name);
  const pathOf = (id: string, revision: number) => `library/grips/${id}/r${revision}.grip.json`;
  const sizeOf = (grip: GripFile) => JSON.stringify(grip, null, 2).length + 1;
  const summary = (e: StoredGrip): LibraryGripSummary => {
    const newest = e.files.get(latestOf(e));
    return {
      id: e.entry.id,
      uid: e.entry.uid,
      name: e.entry.name,
      version: versionOf(e.entry),
      latest: newest?.revision ?? 0,
      revisions: e.entry.revisions,
      weapon: newest?.weapon ?? '',
      group: weaponGroup(newest?.weapon ?? ''),
      hands: newest?.hands ?? null,
      handsWords: newest ? handsWords(newest) : '',
      pose: newest?.pose ?? {},
      updatedAt: e.entry.updatedAt,
      problem: newest ? null : `The newest revision of ${e.entry.name} could not be read.`,
    };
  };
  /** What saving `grip` to `entry` would do, as the core plans it. */
  const plan = (sent: GripFile, entry: string | null): GripSavePlan => {
    const grip: GripFile = {
      ...sent,
      version: GRIP_VERSION,
      name: sent.name.trim(),
      weapon: sent.weapon.trim().toLowerCase(),
    };
    checkWeapon(grip.weapon);
    if (entry === null) {
      const named = { ...grip, revision: 1, name: uniqueName(grip.name, names()) };
      check(named);
      return {
        entry: null,
        entryName: named.name,
        entryVersion: null,
        latest: null,
        revision: 1,
        file: pathOf(freeId(named.name), 1),
        bytes: sizeOf(named),
        grip: named,
        changes: gripChanges(null, named),
        same: false,
      };
    }
    const found = entryOf(entry);
    const newest = fileAt(found, null);
    const revision = Math.max(found.entry.lastRevision, newest.revision) + 1;
    const next = { ...grip, name: found.entry.name, revision };
    check(next);
    return {
      entry: found.entry.id,
      entryName: found.entry.name,
      entryVersion: versionOf(found.entry),
      latest: newest.revision,
      revision,
      file: pathOf(found.entry.id, revision),
      bytes: sizeOf(next),
      grip: next,
      changes: gripChanges(newest, next),
      same: samePose(newest, next),
    };
  };

  return {
    grip_library_list: (): LibraryGripSummary[] =>
      [...entries().values()]
        .map(summary)
        .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase())),
    grip_library_get: ({ entry, revision }: Args): LibraryGripDoc => {
      const found = entryOf(entry);
      const grip = fileAt(found, revision == null ? null : Number(revision));
      return { entry: found.entry, version: versionOf(found.entry), revision: grip.revision, grip };
    },
    grip_library_plan_save: ({ grip, entry }: Args): GripSavePlan =>
      plan(grip as GripFile, entry == null ? null : String(entry)),
    grip_library_save: ({ save }: Args): GripSaved => {
      const { grip, entry, expected, source } = save as GripSave;
      const planned = plan(grip, entry);
      const at = clock.now().toISOString();
      let stored: StoredGrip;
      if (entry === null) {
        const id = freeId(planned.grip.name);
        stored = {
          entry: {
            version: 1,
            id,
            uid: newGripUid(),
            name: planned.grip.name,
            createdAt: at,
            updatedAt: at,
            lastRevision: 1,
            revisions: [gripRevision(1, at, source)],
          },
          files: new Map([[1, planned.grip]]),
        };
        entries().set(id, stored);
      } else {
        stored = entryOf(entry);
        if (expected == null)
          fail(
            'invalid',
            `Saving a revision of ${stored.entry.name} needs the version its plan read.`
          );
        checkExpected(stored, expected);
        if (planned.same)
          fail(
            'invalid',
            `The grip holds what r${planned.latest} of ${stored.entry.name} holds already, so there is nothing to save.`
          );
        const next: LibraryGripEntry = {
          ...stored.entry,
          updatedAt: at,
          lastRevision: planned.revision,
          revisions: [...stored.entry.revisions, gripRevision(planned.revision, at, source)],
        };
        stored.entry = next;
        stored.files.set(planned.revision, planned.grip);
      }
      if (source) currentGripSaveAsks()?.close(source.project, source.recipe, source.layer);
      changed();
      return {
        entry: stored.entry.id,
        name: stored.entry.name,
        revision: planned.revision,
        entryVersion: versionOf(stored.entry),
      };
    },
    grip_library_remove_revision: ({ entry, revision, expected }: Args): LibraryGripEntry => {
      const found = entryOf(entry);
      checkExpected(found, expected);
      const at = Number(revision);
      if (!found.entry.revisions.some(r => r.revision === at))
        fail('not-found', `Not found: revision r${at} of the library grip ${found.entry.id}`);
      if (found.entry.revisions.length === 1)
        fail(
          'invalid',
          `r${at} is the only revision of ${found.entry.name}. Remove the whole grip instead.`
        );
      found.entry = {
        ...found.entry,
        updatedAt: clock.now().toISOString(),
        revisions: found.entry.revisions.filter(r => r.revision !== at),
      };
      found.files.delete(at);
      changed();
      return found.entry;
    },
    grip_library_remove: ({ entry, expected }: Args): string => {
      const found = entryOf(entry);
      checkExpected(found, expected);
      entries().delete(found.entry.id);
      changed();
      return `.removed/${found.entry.id}-${clock.nowMs()}`;
    },
  };
}
