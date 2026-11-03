/**
 * Game mechanics overlaps in mock mode: the core's rules
 * (crates/maquette-model/src/plans/overlap.rs) in TypeScript, tested on
 * the same cases (overlap/cases.json). Two system cards overlap when a
 * `Verb:`, `Look:` or `Input:` line boils down to the same main word:
 * the first content word of a verb, the last of a look, each key of an
 * input with the gesture left out, plurals read as singular.
 */
import type { FineMark, Overlap, OverlapPart, PlanFile, PlanNode } from '../client';
import { STARTER_VALUES } from './plan-presets';

const MAX_OVERLAPS = 500;

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

/** Words in groups split at `, / + & ;` and the words `and` and `or`. */
function groups(value: string): string[][] {
  const out: string[][] = [[]];
  for (const piece of value.toLowerCase().split(/([,/+&;])|[^\p{L}\p{N},/+&;]+/u)) {
    if (piece === undefined || piece === '') continue;
    if (/^[,/+&;]$/.test(piece) || piece === 'and' || piece === 'or') out.push([]);
    else out[out.length - 1]!.push(piece);
  }
  return out;
}

/** The main words of one line's value: one for a verb or a look, one per key for an input. */
export function mainWords(part: OverlapPart, value: string): string[] {
  const content = (words: string[], dropGestures: boolean) =>
    words.filter(w => !STOP_WORDS.has(w) && !(dropGestures && GESTURES.has(w))).map(singular);
  if (part === 'input') {
    const keys = groups(value)
      .map(g => content(g, true).join(' '))
      .filter(Boolean);
    return [...new Set(keys)].sort();
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
    const part = line.slice(0, at).trim().toLowerCase();
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
const byCode = (x: string, y: string) => (x < y ? -1 : x > y ? 1 : 0);

/** A Game mechanics canvas's overlaps in node id order, each marked fine for the words it holds now. */
export function overlapsOf(plan: Pick<PlanFile, 'preset' | 'nodes'> & { fine?: FineMark[] }) {
  if (plan.preset !== 'mechanics') return [];
  const byWord = new Map<string, { part: OverlapPart; word: string; ids: Set<string> }>();
  for (const node of plan.nodes)
    for (const [part, word] of cardWords(node)) {
      const key = `${PART_ORDER[part]}\u0000${word}`;
      const found = byWord.get(key) ?? { part, word, ids: new Set<string>() };
      found.ids.add(node.id);
      byWord.set(key, found);
    }
  const found = new Map<string, Overlap>();
  for (const key of [...byWord.keys()].sort(byCode)) {
    const { part, word, ids } = byWord.get(key)!;
    const sorted = [...ids].sort(byCode);
    sorted.forEach((a, i) => {
      for (const b of sorted.slice(i + 1)) {
        const pair = `${a}\u0000${b}\u0000${PART_ORDER[part]}`;
        if (!found.has(pair)) found.set(pair, { a, b, part, word, fine: false });
      }
    });
  }
  const node = (id: string) => plan.nodes.find(n => n.id === id);
  return [...found.keys()]
    .sort(byCode)
    .slice(0, MAX_OVERLAPS)
    .map(k => {
      const o = found.get(k)!;
      const na = node(o.a);
      const nb = node(o.b);
      const key = na && nb ? pairKey(na, nb) : '';
      return {
        ...o,
        fine: (plan.fine ?? []).some(m => m.a === o.a && m.b === o.b && m.key === key),
      };
    });
}
