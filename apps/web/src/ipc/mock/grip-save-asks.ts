/**
 * Grip save approvals in mock mode: an agent's propose_grip_save waits as a
 * card until the person saves the grip in the save a grip sheet (the save
 * closes it) or the card's Deny turns it down. No answer to the card writes
 * anything. `?agents=grip-save` (with `?library=demo`) opens with one
 * waiting: the hands of Scythe charge, held as r3 of the scythe's staff
 * grip. The fake agents make them; the grip library handlers reach the ones
 * of the backend made last.
 */
import type { Approval } from '../client';
import { clock } from '@/lib/clock';
import { GRIP_DEMO, demoGrips } from './grip-library-fixtures';

export type GripSaveAsks = ReturnType<typeof createGripSaveAsks>;

let current: GripSaveAsks | null = null;

/** The approvals of the backend made last, if it has agents. */
export const currentGripSaveAsks = (): GripSaveAsks | null => current;

/** The demo's proposal: both hands of the scythe recipe as the next revision of its grip. */
function demoAsk(): Approval {
  const scythe = demoGrips().get('scythe-staff-grip')!;
  const newest = scythe.files.get(2)!;
  return {
    id: 'grip-save-approval-1',
    kind: 'grip-save',
    project: 'frostveil',
    runId: null,
    runNumber: null,
    step: null,
    job: GRIP_DEMO,
    assetName: 'Scythe, staff grip r3',
    service: null,
    detail: 'From the pose keys on both hands in Scythe charge, held, at frame 30.',
    items: [],
    requestedAtMs: clock.nowMs() - 30_000,
    recipeSave: {
      recipe: GRIP_DEMO,
      entry: 'scythe-staff-grip',
      revision: 3,
      grip: {
        layer: 'l-1',
        frame: 30,
        name: newest.name,
        weapon: newest.weapon,
        hands: newest.hands,
        bones: Object.keys(newest.pose),
        latest: 2,
        file: 'library/grips/scythe-staff-grip/r3.grip.json',
      },
    },
  };
}

export function createGripSaveAsks(changed: () => void, demo = false) {
  let asks: Approval[] = demo ? [demoAsk()] : [];
  const made = {
    approvals: (): Approval[] => asks,
    /** The person saved a grip from that pose key change in the sheet: nothing waits any more. */
    close(project: string, recipe: string, layer: string) {
      const left = asks.filter(
        a => a.project !== project || a.job !== recipe || a.recipeSave?.grip?.layer !== layer
      );
      if (left.length === asks.length) return;
      asks = left;
      changed();
    },
    /** The card's Deny: false when `id` is no grip save approval. */
    deny(id: string): boolean {
      if (!asks.some(a => a.id === id)) return false;
      asks = asks.filter(a => a.id !== id);
      changed();
      return true;
    },
  };
  current = made;
  return made;
}
