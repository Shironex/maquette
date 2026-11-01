import type { Aspect, AssetKind, ConceptEvent, LabView } from '@/ipc/client';

export interface Draft {
  name: string;
  prompt: string;
  /** Relative to the project folder, or absolute. */
  refs: string[];
  aspect: Aspect;
  count: number;
  /** Whether the jobs carry the project's style board. */
  useStyle: boolean;
}

/** "Make object refs...": the concept to take objects from and what to make. */
export interface ObjectsDraft {
  project: string;
  /** Relative to the project folder. */
  source: string;
  /** The same file, absolute, for the picture. */
  sourcePath: string;
  /** The direction or asset the concept belongs to. */
  sourceName: string;
  objects: string[];
  /** Extra instructions for the prompt, by object name. Not part of the asset name. */
  notes: Record<string, string>;
  aspect: Aspect;
  kind: 'prop' | 'character';
  /** Whether the jobs carry the project's style board. */
  useStyle: boolean;
}

/** The style board as edited, saved in one go. */
export interface StyleDraft {
  line: string;
  /** `#rrggbb` colours. */
  palette: string[];
  /** Board files as they are (`style/...`), concepts and refs of the project, or absolute paths. */
  images: string[];
}

export interface LabStore {
  view: LabView | null;
  loading: boolean;
  /** The last read of the concepts failed. */
  loadFailed: boolean;
  /** The direction shown in the results. */
  selected: string | null;
  draft: Draft;
  /** The image open in the lightbox, as an absolute path. */
  lightbox: string | null;
  /** What is running: `generate`, `sheet`, `send`, `move`, `objects`, `style` or null. */
  busy: string | null;
  load: (project: string) => Promise<void>;
  select: (direction: string) => void;
  /** Loads an earlier direction's brief into the composer. */
  reuse: (direction: string) => void;
  setDraft: (patch: Partial<Draft>) => void;
  /** Clears the brief for a new direction. */
  newDraft: () => void;
  addRefs: (paths: string[]) => Promise<void>;
  removeRef: (path: string) => void;
  generate: () => Promise<void>;
  cancel: (jobId: string) => Promise<void>;
  /** Runs a failed or cancelled job again: the same prompt, refs and file. */
  retry: (jobId: string) => Promise<void>;
  cancelAll: () => Promise<void>;
  /** Run next: the open project's queued jobs (or `jobIds`) go to the front of the line. */
  runNext: (jobIds?: string[]) => Promise<void>;
  clearFinished: () => Promise<void>;
  mark: (file: string, marks: { starred?: boolean; picked?: boolean }) => Promise<void>;
  contactSheet: () => Promise<void>;
  /** Copies the picks into refs/; resolves to the asset's id, or null when it failed. */
  /** Copies the picks into refs/. `quiet` skips the toast, for when the Send to 3D panel opens next. */
  sendPicks: (kind: AssetKind, quiet?: boolean) => Promise<string | null>;
  /** Takes the Pick mark off every picked concept of `direction`. */
  clearPicks: (direction: string) => Promise<void>;
  /** Moves an earlier direction, with its refs and Board card, into another project. */
  moveDirection: (direction: string, to: string) => Promise<boolean>;
  openLightbox: (path: string | null) => void;
  onEvent: (event: ConceptEvent) => void;
  objects: ObjectsDraft | null;
  openObjects: (
    draft: Pick<ObjectsDraft, 'project' | 'source' | 'sourcePath' | 'sourceName'>
  ) => void;
  setObjects: (
    patch: Partial<Pick<ObjectsDraft, 'objects' | 'notes' | 'aspect' | 'kind' | 'useStyle'>>
  ) => void;
  closeObjects: () => void;
  /** Queues one Codex job per object; resolves true when they started. */
  makeObjects: () => Promise<boolean>;
  /**
   * Sends the picks of an objects direction to Refs, one asset per pick.
   * Resolves to the new assets' ids, or null when it failed.
   */
  sendObjectPicks: (quiet?: boolean) => Promise<string[] | null>;
  /** The style board dialog's draft, or null while it is closed. */
  style: StyleDraft | null;
  /** Opens the style board dialog with the board as it is. */
  openStyle: () => void;
  setStyle: (patch: Partial<StyleDraft>) => void;
  closeStyle: () => void;
  /** Saves the draft as the board; resolves true when it was saved. */
  saveStyle: () => Promise<boolean>;
  /** Adds one concept or ref of the project to the board straight away. */
  addToStyle: (file: string) => Promise<void>;
}
