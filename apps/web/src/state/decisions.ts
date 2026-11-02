/**
 * Who answered an approval: what a card's Approve or Deny says about the
 * click, and the words the Credits view shows for the record it left.
 */
import type {
  DecidedBy,
  Decision,
  DecisionInput,
  DecisionRecord,
  DecisionSource,
} from '@/ipc/client';

/** The part of a click the record keeps. */
interface Press {
  /** 0 for Enter or Space on a focused button (and for a script's click). */
  detail: number;
  /** False when a script made the click. */
  isTrusted: boolean;
}

/** What an answer from `source` says about itself, from the click when there is one. */
export function decidedBy(source: DecisionSource, press?: Press): DecidedBy {
  const input: DecisionInput | null = press ? (press.detail === 0 ? 'keyboard' : 'pointer') : null;
  return {
    source,
    input,
    trusted: press ? press.isTrusted : null,
    pageFocused: document.hasFocus(),
    pageVisible: document.visibilityState === 'visible',
  };
}

const VERB: Record<Decision, string> = {
  approved: 'approved',
  denied: 'denied',
  expired: 'expired',
  cancelled: 'cancelled',
};

const WHERE: Record<DecisionSource, string> = {
  'floating-card': 'in the floating card',
  'agents-panel': 'in Settings > Agents',
  'ship-view': 'in the Ship view',
  avatar: 'in the Avatar view',
  'rig-doctor': 'in the Rig doctor',
  'jobs-cancelled': 'as its jobs were cancelled',
  'run-cancelled': 'as its run was cancelled',
  'question-changed': 'as the command changed',
  quit: 'as Maquette quit',
  'workspace-changed': 'as another workspace opened',
  unknown: 'in the app',
};

const INPUT: Record<DecisionInput, string> = {
  pointer: 'with the mouse or trackpad',
  keyboard: 'with the keyboard',
};

const two = (n: number) => String(n).padStart(2, '0');

/** `approved 14:02 in the floating card`. */
export function decisionText(record: DecisionRecord): string {
  const at = new Date(record.atMs ?? 0);
  return `${VERB[record.decision]} ${two(at.getHours())}:${two(at.getMinutes())} ${WHERE[record.by.source]}`;
}

/** What the window said about itself, in plain sentences. */
function windowLine(by: DecidedBy): string | null {
  if (by.windowMinimized) return 'The Maquette window was minimised.';
  if (by.windowVisible === false) return 'The Maquette window was hidden.';
  if (by.windowFocused === false) return 'The Maquette window was not focused.';
  if (by.windowFocused) return 'The Maquette window was focused.';
  return null;
}

/** The whole record in a few sentences, for the row's tooltip. */
export function decisionTitle(record: DecisionRecord): string {
  const { by } = record;
  const verb = VERB[record.decision];
  const at = new Date(record.atMs ?? 0).toLocaleString('en-GB');
  const after = Math.max(0, Math.round(((record.atMs ?? 0) - (record.requestedAtMs ?? 0)) / 1000));
  const how = [
    by.input ? INPUT[by.input] : null,
    by.trusted === false ? 'by a script, not from a person' : null,
  ].filter(Boolean);
  const first = `${verb[0]!.toUpperCase()}${verb.slice(1)} ${at} ${WHERE[by.source]}, ${after} s after the agent asked${how.length ? `, ${how.join(', ')}` : ''}.`;
  return [first, windowLine(by)].filter(Boolean).join(' ');
}
