/**
 * Game mechanics overlaps in mock mode: the core's rules
 * (crates/maquette-model/src/plans/overlap.rs) in TypeScript, tested on
 * the same cases (overlap/cases.json). Two system cards overlap when a
 * `Verb:`, `Look:` or `Input:` line boils down to the same main word:
 * the first content word of a verb, the last of a look, each key of an
 * input with the gesture left out, plurals read as singular. Words are
 * made as Rust's `char::is_alphanumeric` and `char::to_lowercase` make them,
 * a character at a time, and strings compare by code point as Rust's do. The
 * search is bounded the same way: a card's first 32 main words, and it stops
 * at 500 pairs in node id order.
 */
import type { FineMark, Overlap, OverlapPart, PlanFile, PlanNode } from '../client';
import { STARTER_VALUES } from './plan-presets';

const MAX_OVERLAPS = 500;
const MAX_CARD_WORDS = 32;

const STOP_WORDS = new Set(
  'a an the to of and or for with on in at by from into onto your you it its their them is are be as up out over around then than each every'.split(
    ' '
  )
);
const GESTURES = new Set(
  'tap hold press click double long quick release mash push held pressed'.split(' ')
);

/** `word` as its singular: `rings` to `ring`, `bodies` to `body`; `glass` and `bus` stay. */
export function singular(word: string): string {
  const n = [...word].length;
  if (n > 4 && word.endsWith('ies')) return `${word.slice(0, -3)}y`;
  for (const end of ['sses', 'xes', 'zes', 'ches', 'shes'])
    if (n > end.length && word.endsWith(end)) return word.slice(0, -2);
  if (n > 3 && word.endsWith('s') && !['ss', 'us', 'is'].some(e => word.endsWith(e)))
    return word.slice(0, -1);
  return word;
}

/** A letter or digit as Rust's `char::is_alphanumeric` says: Alphabetic or a number. */
const WORD_CHAR = /^[\p{Alphabetic}\p{N}]$/u;
const SPLITS = new Set([',', '/', '+', '&', ';']);

/** Words in groups split at `, / + & ;` and the words `and` and `or`, each character lower cased alone. */
function groups(value: string): string[][] {
  const out: string[][] = [[]];
  let word = '';
  const endWord = () => {
    if (!word) return;
    if (word === 'and' || word === 'or') out.push([]);
    else out[out.length - 1]!.push(word);
    word = '';
  };
  for (const c of value) {
    if (WORD_CHAR.test(c)) {
      word += c.toLowerCase();
      continue;
    }
    endWord();
    if (SPLITS.has(c)) out.push([]);
  }
  endWord();
  return out;
}

/** Strings in code point order, as Rust compares them. */
export function byCode(x: string, y: string): number {
  const a = [...x];
  const b = [...y];
  for (let i = 0; i < Math.min(a.length, b.length); i += 1) {
    const d = a[i]!.codePointAt(0)! - b[i]!.codePointAt(0)!;
    if (d !== 0) return d;
  }
  return a.length - b.length;
}

/** The main words of one line's value: one for a verb or a look, one per key for an input. */
export function mainWords(part: OverlapPart, value: string): string[] {
  const content = (words: string[], dropGestures: boolean) =>
    words.filter(w => !STOP_WORDS.has(w) && !(dropGestures && GESTURES.has(w))).map(singular);
  if (part === 'input') {
    const keys = groups(value)
      .map(g => content(g, true).join(' '))
      .filter(Boolean);
    return [...new Set(keys)].sort(byCode);
  }
  const words = content(groups(value).flat(), false);
  const main = part === 'verb' ? words[0] : words[words.length - 1];
  return main === undefined ? [] : [main];
}

const STARTERS = new Set(STARTER_VALUES.map(v => v.toLowerCase()));

function cardWords(node: PlanNode): [OverlapPart, string][] {
  if (node.kind !== 'note') return [];
  return node.text.split('\n').flatMap(line => {
    const at = line.indexOf(':');
    if (at < 0) return [];
    const part = line
      .slice(0, at)
      .trim()
      .replace(/[A-Z]/g, ch => ch.toLowerCase());
    if (part !== 'verb' && part !== 'look' && part !== 'input') return [];
    const value = line.slice(at + 1);
    if (STARTERS.has(value.trim().toLowerCase())) return [];
    return mainWords(part, value).map(w => [part, w] as [OverlapPart, string]);
  });
}

/** FNV-1a over `text` as UTF-8, eight hex digits. */
function fnv(text: string): string {
  let hash = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(text)) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, '0');
}

/** The key of two cards' words, in id order, as the core makes it. */
export function pairKey(
  a: Pick<PlanNode, 'id' | 'title' | 'text'>,
  b: Pick<PlanNode, 'id' | 'title' | 'text'>
): string {
  const [x, y] = a.id <= b.id ? [a, b] : [b, a];
  return fnv(`${x.title}\n${x.text}\u0000${y.title}\n${y.text}`);
}

const PART_ORDER: Record<OverlapPart, number> = { verb: 0, look: 1, input: 2 };
const byWord = (x: [OverlapPart, string], y: [OverlapPart, string]) =>
  PART_ORDER[x[0]] - PART_ORDER[y[0]] || byCode(x[1], y[1]);

/**
 * A Game mechanics canvas's overlaps in node id order, each marked fine for
 * the words it holds now. Each card pairs only with the cards after it, a
 * card at a time, so the work stops once 500 are found.
 */
export function overlapsOf(plan: Pick<PlanFile, 'preset' | 'nodes'> & { fine?: FineMark[] }) {
  if (plan.preset !== 'mechanics') return [];
  const cards = plan.nodes
    .map(node => {
      const sorted = cardWords(node).sort(byWord);
      const words = sorted
        .filter((w, i) => i === 0 || byWord(w, sorted[i - 1]!) !== 0)
        .slice(0, MAX_CARD_WORDS);
      return { node, words };
    })
    .filter(c => c.words.length > 0)
    .sort((x, y) => byCode(x.node.id, y.node.id));
  const holders = new Map<string, string[]>();
  const keyOf = ([part, word]: [OverlapPart, string]) => `${part}\u0000${word}`;
  for (const { node, words } of cards)
    for (const w of words) {
      const ids = holders.get(keyOf(w)) ?? [];
      ids.push(node.id);
      holders.set(keyOf(w), ids);
    }
  const byId = new Map(cards.map(c => [c.node.id, c.node]));
  const out: Overlap[] = [];
  for (const { node: na, words } of cards) {
    const pairs = new Map<string, { b: string; part: OverlapPart; word: string }>();
    for (const w of words)
      for (const b of holders.get(keyOf(w))!) {
        const pair = `${b}\u0000${PART_ORDER[w[0]]}`;
        if (byCode(b, na.id) > 0 && !pairs.has(pair))
          pairs.set(pair, { b, part: w[0], word: w[1] });
      }
    const sorted = [...pairs.values()].sort(
      (x, y) => byCode(x.b, y.b) || PART_ORDER[x.part] - PART_ORDER[y.part]
    );
    for (const { b, part, word } of sorted) {
      const key = pairKey(na, byId.get(b)!);
      const fine = (plan.fine ?? []).some(m => m.a === na.id && m.b === b && m.key === key);
      out.push({ a: na.id, b, part, word, fine });
      if (out.length === MAX_OVERLAPS) return out;
    }
  }
  return out;
}
