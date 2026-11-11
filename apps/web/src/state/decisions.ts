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
  'recipe-review': 'in the recipe review',
  'recipe-save': 'in the save to library sheet',
  'grip-save': 'in the save a grip sheet',
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

/** What the operating system said about the window, in a plain sentence. */
function windowLine(by: DecidedBy): string | null {
  const state = by.windowMinimized
    ? 'minimised'
    : by.windowVisible === false
      ? 'hidden'
      : by.windowFocused === false
        ? 'not focused'
        : by.windowFocused
          ? 'focused'
          : null;
  return state ? `The system reported the Maquette window was ${state}.` : null;
}

/** What the app's own page said about the press, in a plain sentence. */
function pressLine(by: DecidedBy): string | null {
  const parts = [
    by.input ? `a press ${INPUT[by.input]}` : null,
    by.trusted === false ? 'the page reported it as made by a script' : null,
  ].filter(Boolean);
  if (!parts.length) return null;
  const line = parts.join('; ');
  return line.startsWith('a press')
    ? `The page reported ${line}.`
    : `${line[0]!.toUpperCase()}${line.slice(1)}.`;
}

/**
 * The whole record in a few sentences, for the row's tooltip. What the page
 * reports (the card, the input, a script's click) is the app's own word,
 * kept apart from what the operating system reports about the window.
 */
export function decisionTitle(record: DecisionRecord): string {
  const { by } = record;
  const verb = VERB[record.decision];
  const at = new Date(record.atMs ?? 0).toLocaleString('en-GB');
  const after = Math.max(0, Math.round(((record.atMs ?? 0) - (record.requestedAtMs ?? 0)) / 1000));
  const first = `${verb[0]!.toUpperCase()}${verb.slice(1)} ${at} ${WHERE[by.source]}, ${after} s after the agent asked.`;
  return [first, pressLine(by), windowLine(by)].filter(Boolean).join(' ');
}
