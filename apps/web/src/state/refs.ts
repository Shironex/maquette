/**
 * An asset's refs and their views, the web side of `maquette_core::refs`:
 * the four view slots in send order, the swap a tag change makes, the
 * order a send carries the images in, and the candidates the Lab drew.
 */
import type {
  Asset,
  AssetRef,
  ConceptJob,
  Direction,
  RefFix,
  RefView,
  RefWarning,
} from '@/ipc/client';

/** The view slots, in the order a send carries them. */
export const SLOTS = ['front', 'left', 'back', 'right'] as const satisfies readonly RefView[];

export const VIEW_LABEL: Record<RefView, string> = {
  front: 'Front',
  left: 'Left',
  back: 'Back',
  right: 'Right',
  other: 'Other',
};

/** The views a drawn candidate can fill: every slot but the front. */
export const DRAWN = ['left', 'back', 'right'] as const satisfies readonly RefView[];

/**
 * Gives `file` the view `view`. A slot holds one ref, so the ref that held
 * it takes `file`'s old view (other when `file` is new), as the core does.
 */
export function assign(refs: readonly AssetRef[], file: string, view: RefView): AssetRef[] {
  const old = refs.find(r => r.file === file)?.view ?? 'other';
  const next = refs.map(r =>
    r.file === file ? { ...r, view } : view !== 'other' && r.view === view ? { ...r, view: old } : r
  );
  return next.some(r => r.file === file) ? next : [...next, { file, view }];
}

/** The ref in each slot, or null for an empty one. */
export function slots(refs: readonly AssetRef[]): Record<(typeof SLOTS)[number], string | null> {
  const at = (view: RefView) => refs.find(r => r.view === view)?.file ?? null;
  return { front: at('front'), left: at('left'), back: at('back'), right: at('right') };
}

export function filledCount(refs: readonly AssetRef[]): number {
  return SLOTS.filter(v => refs.some(r => r.view === v)).length;
}

/** "1 of 4 views". */
export function viewsLine(refs: readonly AssetRef[]): string {
  return `${filledCount(refs)} of 4 views`;
}

/** The slots still empty, the ones Make the other views draws by default. */
export function emptyViews(refs: readonly AssetRef[]): RefView[] {
  return DRAWN.filter(v => !refs.some(r => r.view === v));
}

/** The view an image goes as in a send of `images`, or null when it stays home. */
export function sentView(asset: Asset, images: readonly string[], file: string): RefView | null {
  const tags = asset.refs ?? [];
  const tagged = images.some(i => tags.some(r => r.file === i && r.view !== 'other'));
  if (!tagged) return SLOTS[images.indexOf(file)] ?? null;
  const view = tags.find(r => r.file === file)?.view;
  return view && view !== 'other' ? view : null;
}

/** The images of `images` that a send carries, in the order it carries them. */
export function sendOrder(asset: Asset, images: readonly string[]): string[] {
  return SLOTS.flatMap(view => images.filter(i => sentView(asset, images, i) === view));
}

/** A Lab candidate made for the asset's refs, with the view or ref it is for. */
export interface Candidate {
  file: string;
  /** "left", "back" or "right" for a view; the cleaned ref's file for a clean copy. */
  forWhat: string;
  /** Its copy in `refs/` once it was used. */
  used: string | null;
}

/** The candidates of the asset's views or clean directions, newest last. */
export function candidates(directions: readonly Direction[], kind: 'views' | 'clean'): Candidate[] {
  return directions
    .filter(d => d.refsJob?.kind === kind)
    .flatMap(d =>
      d.images.map(i => ({ file: i.file, forWhat: i.title ?? '', used: i.refFile ?? null }))
    );
}

/** The drawn candidates of one view and the jobs still drawing it. */
export interface ViewCandidates {
  images: (Candidate & { inSlot: boolean })[];
  drawing: number;
}

/** The view candidates grouped by the view they were drawn for. */
export function candidatesByView(
  refs: readonly AssetRef[],
  directions: readonly Direction[],
  jobs: readonly ConceptJob[]
): Record<(typeof DRAWN)[number], ViewCandidates> {
  const all = candidates(directions, 'views');
  const live = jobs.filter(j => j.state === 'queued' || j.state === 'running');
  const of = (view: RefView): ViewCandidates => ({
    images: all
      .filter(c => c.forWhat === view)
      .map(c => ({ ...c, inSlot: refs.some(r => r.file === c.used && r.view === view) })),
    drawing: live.filter(j => j.title === view).length,
  });
  return { left: of('left'), back: of('back'), right: of('right') };
}

/** Warnings about one image, for the flag on its tile. */
export function warningsFor(warnings: readonly RefWarning[], file: string): RefWarning[] {
  return warnings.filter(w => w.file === file);
}

export const FIX_LABEL: Record<RefFix, string> = {
  'clean-background': 'Clean background',
  'make-views': 'Make the other views',
  'leave-out': 'Leave out this image',
};

/**
 * A send's warnings in two lists: the weak ref ones, and the note that the
 * asset has a model already (the send makes its next version), which is no
 * fault of the refs and does not count as a ref warning.
 */
export function splitModelNote(warnings: readonly RefWarning[]): {
  weak: RefWarning[];
  note: RefWarning | null;
} {
  return {
    weak: warnings.filter(w => w.kind !== 'has-model'),
    note: warnings.find(w => w.kind === 'has-model') ?? null,
  };
}

/** "3 ref warnings". */
export function warningsLine(count: number): string {
  return `${count} ref ${count === 1 ? 'warning' : 'warnings'}`;
}
