/**
 * Splitting a model into parts three ways (free in Blender, on Tripo, on
 * Meshy) and assembling one prop from part models: the settings, what each
 * way costs, the confirm line and the checks, the way the core reads them.
 */
import type {
  AssemblePart,
  Granularity,
  MeshySplit,
  MeshySplitMode,
  Pivot,
  ProviderJob,
  SegmentModel,
  SplitPrices,
  SplitSource,
  TripoSplit,
} from '@/ipc/client';
import type { SplitAfter } from '@/ipc/client';
import { estimateText } from './credits';

/** Where a split runs. */
export type SplitWay = 'blender' | 'tripo' | 'meshy';

export const SPLIT_WAYS: readonly { value: SplitWay; label: string }[] = [
  { value: 'blender', label: 'In Blender, free' },
  { value: 'tripo', label: 'Tripo' },
  { value: 'meshy', label: 'Meshy' },
];

export const SEGMENT_MODELS: readonly { value: SegmentModel; label: string }[] = [
  { value: 'geometry', label: 'Geometry' },
  { value: 'semantic', label: 'Semantic, beta' },
];

export const GRANULARITIES: readonly { value: Granularity; label: string }[] = [
  { value: 'simple', label: 'Simple' },
  { value: 'balanced', label: 'Balanced' },
  { value: 'detailed', label: 'Detailed' },
];

export const MESHY_MODES: readonly { value: MeshySplitMode; label: string }[] = [
  { value: 'auto', label: 'Auto' },
  { value: 'by-parts', label: 'By parts' },
  { value: 'by-color', label: 'By colour' },
];

export const PIVOTS: readonly { value: Pivot; label: string }[] = [
  { value: 'bottom-centre', label: 'Bottom centre' },
  { value: 'centre', label: 'Centre' },
  { value: 'left-hinge', label: 'Left hinge' },
  { value: 'right-hinge', label: 'Right hinge' },
  { value: 'origin', label: 'Its own origin' },
];

/** Meshy's Retexture, as the core prices it. */
export const MESHY_RETEXTURE_CREDITS = 10;
/** Meshy reads a texture prompt of at most this many characters. */
export const MAX_RETEXTURE_PROMPT = 800;

/** What a re-texture's prompt lacks, as a sentence. */
export function retextureProblem(prompt: string): string | null {
  const words = prompt.trim();
  if (!words) return 'Say what the new texture should look like.';
  if (words.length > MAX_RETEXTURE_PROMPT) {
    return `Keep it under ${MAX_RETEXTURE_PROMPT} characters; Meshy refuses longer ones.`;
  }
  return null;
}

/** The clause on what follows a Meshy split, for its summary and its question. */
export function afterLine(after: SplitAfter, textures: number): string {
  if (textures <= 0) return '';
  if (after === 'grey') return ', then the parts stay grey';
  if (after === 'retexture') return ', then a re-texture on Meshy';
  return ', then a bake back in Blender';
}

/** Meshy looks for at most this many named parts. */
export const MAX_MESHY_PARTS = 10;
/** The part names together, as Meshy's prompt, stay under this. */
export const MAX_MESHY_PROMPT = 600;
export const MIN_ASSEMBLE_PARTS = 2;
export const MAX_ASSEMBLE_PARTS = 16;
/** A part or prop name, as the export node names take it. */
export const MAX_NODE_NAME = 64;

export const DEFAULT_TRIPO: TripoSplit = {
  model: 'geometry',
  granularity: 'balanced',
  splitByConnectivity: true,
  complete: null,
};

export const DEFAULT_MESHY: MeshySplit = { mode: 'auto', parts: [] };

/** The names trimmed, without blanks or repeats, as the core sends them. */
export function meshyNames(parts: readonly string[]): string[] {
  const out: string[] = [];
  for (const name of parts.map(p => p.trim())) {
    if (name && !out.some(n => n.toLowerCase() === name.toLowerCase())) out.push(name);
  }
  return out;
}

/** What is wrong with a Meshy split's settings, as a sentence. */
export function meshyProblem(split: MeshySplit): string | null {
  if (split.mode === 'auto') return null;
  const names = meshyNames(split.parts ?? []);
  if (names.length === 0) return 'Name at least one part for Meshy to look for.';
  if (names.length > MAX_MESHY_PARTS) return `Meshy looks for at most ${MAX_MESHY_PARTS} parts.`;
  if (names.join(', ').length > MAX_MESHY_PROMPT) {
    return `Keep the part names under ${MAX_MESHY_PROMPT} characters together; Meshy refuses longer ones.`;
  }
  return null;
}

/** The credits a way should cost, from the providers' published numbers. */
export function splitCredits(way: SplitWay, tripo: TripoSplit, prices: SplitPrices): number | null {
  if (way === 'blender') return 0;
  if (way === 'meshy') return prices.meshySplit;
  const segment = prices.tripoSegment ?? 0;
  return tripo.complete ? segment + (prices.tripoComplete ?? 0) : segment;
}

const MODEL_WORD: Record<SegmentModel, string> = { geometry: 'geometry', semantic: 'semantic' };

/** "1 segment task, semantic model, balanced" and on. */
export function tripoLine(t: TripoSplit): string {
  const tasks = t.complete ? '1 segment task and 1 completion task' : '1 segment task';
  const grain = t.model === 'semantic' ? `, ${t.granularity}` : '';
  return `${tasks}, ${MODEL_WORD[t.model]} model${grain}`;
}

/** "Meshy Auto Split, by parts, 5 names" and on. */
export function meshyLine(m: MeshySplit): string {
  const mode = MESHY_MODES.find(x => x.value === m.mode)?.label.toLowerCase() ?? m.mode;
  if (m.mode === 'auto') return 'Meshy Auto Split, auto';
  const count = meshyNames(m.parts ?? []).length;
  return `Meshy Auto Split, ${mode}, ${count} ${count === 1 ? 'name' : 'names'}`;
}

/** The confirm question's body: what goes out, the cost, and the cancel rule. */
export function confirmLine(
  way: 'tripo' | 'meshy',
  tripo: TripoSplit,
  meshy: MeshySplit,
  prices: SplitPrices,
  textures: number,
  after: SplitAfter = 'bake'
): string {
  const cost = estimateText(splitCredits(way, tripo, prices));
  if (way === 'tripo') {
    return `${tripoLine(tripo)}. ${cost ?? ''}. Tripo cannot cancel once it is sent.`;
  }
  const next = afterLine(after, textures);
  const more =
    after === 'retexture' && textures > 0
      ? ` The re-texture is ${estimateText(MESHY_RETEXTURE_CREDITS)?.toLowerCase() ?? ''} more, asked again when the parts arrive.`
      : '';
  return `${meshyLine(meshy)}${next}. ${cost ?? ''}. You can cancel while Meshy has not started.${more}`;
}

/** "v2" or "the Inbox model": the model a split cuts. */
export function sourceLabel(source: Pick<SplitSource, 'version' | 'item'>): string {
  return source.version !== null ? `v${source.version}` : 'the Inbox model';
}

/** The splits of an asset whose texture bake back needs a word, newest first. */
export function bakeJobs(jobs: readonly ProviderJob[], project: string, asset: string) {
  return jobs
    .filter(j => j.project === project && j.asset === asset && j.split?.bake)
    .sort((a, b) => (b.createdAtMs ?? 0) - (a.createdAtMs ?? 0));
}

/** A part of the prop as the Assemble sheet edits it. */
export interface DraftPart extends AssemblePart {
  /** The model's triangles, for the list. */
  triangles: number;
}

/** What stops an assembly, as a sentence, mirroring the core's checks. */
export function assembleProblem(name: string, parts: readonly DraftPart[]): string | null {
  const prop = name.trim();
  if (!prop || prop.length > MAX_NODE_NAME) {
    return `Give the prop a name of 1 to ${MAX_NODE_NAME} characters.`;
  }
  if (parts.length < MIN_ASSEMBLE_PARTS || parts.length > MAX_ASSEMBLE_PARTS) {
    return `Assemble ${MIN_ASSEMBLE_PARTS} to ${MAX_ASSEMBLE_PARTS} parts, not ${parts.length}.`;
  }
  const seen = new Set<string>();
  for (const [i, part] of parts.entries()) {
    const n = part.name.trim();
    if (!n || n.length > MAX_NODE_NAME) {
      return `Part ${i + 1}: give it a name of 1 to ${MAX_NODE_NAME} characters.`;
    }
    if (n === prop) return `${n} is the prop's own name. Give the part another.`;
    if (seen.has(n)) return `Two parts are named ${n}. Give each its own name.`;
    seen.add(n);
  }
  return null;
}

/** A node name from an asset name: "Sigil gate door" to "sigil_gate_door". */
export function nodeName(name: string): string {
  return (
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^_+|_+$/g, '') || 'part'
  );
}

/** Moves the part at `from` one place up (-1) or down (1). */
export function movePart<T>(parts: readonly T[], from: number, by: -1 | 1): T[] {
  const to = from + by;
  if (to < 0 || to >= parts.length) return [...parts];
  const next = [...parts];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved as T);
  return next;
}
