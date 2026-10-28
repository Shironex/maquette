import type {
  BatchOptions,
  DownloadsScan,
  KeyStatus,
  KeyTest,
  MeshyPreset,
  PresetVariant,
  ProviderEvent,
  ProviderId,
  ProviderJob,
  SendOptions,
  TripoPreset,
} from '@/ipc/client';
import type { SendDraft } from './providers';
import type { BatchDraft } from './batch';

export interface Panel {
  project: string;
  asset: string;
  options: SendOptions | null;
  draft: SendDraft | null;
  /** The draft as the panel opened with it, to tell unsent edits. */
  initial: SendDraft | null;
  /** A By hand send that just wrote its folder: the panel shows its card. */
  handoff: ProviderJob | null;
  error: string | null;
  /** A redo from the Inbox: the earlier send it repeats. */
  redo?: ProviderJob | null;
}

export interface BatchPanel {
  project: string;
  assets: string[];
  options: BatchOptions | null;
  draft: BatchDraft | null;
  /** The draft as the sheet opened with it, to tell unsent edits. */
  initial: BatchDraft | null;
  /** An API batch asks once before it spends credits. */
  confirming: boolean;
  error: string | null;
  /** A redo from the Inbox: each asset starts from its earlier send. */
  redo?: boolean;
}

export interface Downloads {
  project: string;
  scan: DownloadsScan | null;
  error: string | null;
  /** File path to the send it goes to, as the user set it. */
  choices: Record<string, string | null>;
  /** Files already handed back. */
  used: string[];
}

export interface ProvidersStore {
  project: string | null;
  jobs: ProviderJob[];
  keys: KeyStatus[];
  panel: Panel | null;
  /** The queue sheet is open. */
  queueOpen: boolean;
  sending: boolean;
  /** Job ids being cancelled or handed back right now. */
  busy: string[];
  /** The service whose key is being saved, removed or tested. */
  keyBusy: ProviderId | null;
  load: (project: string) => Promise<void>;
  onEvent: (event: ProviderEvent) => void;
  openPanel: (project: string, asset: string) => Promise<void>;
  closePanel: () => void;
  setDraft: (patch: Partial<SendDraft>) => void;
  /** Starts over from the saved preset for the draft's variant. */
  resetDraft: () => void;
  send: () => Promise<void>;
  showHandoff: (job: ProviderJob) => void;
  setQueueOpen: (open: boolean) => void;
  cancel: (id: string) => Promise<void>;
  handBack: (id: string, file: string) => Promise<void>;
  openHandoff: (id: string) => Promise<void>;
  clearFinished: () => Promise<void>;
  loadKeys: () => Promise<void>;
  saveKey: (service: ProviderId, key: string) => Promise<boolean>;
  removeKey: (service: ProviderId) => Promise<void>;
  testKey: (service: ProviderId, key: string | null) => Promise<KeyTest | null>;
  batch: BatchPanel | null;
  /** Counts the batches that went out, so the Board can drop its selection. */
  batchesSent: number;
  openBatch: (project: string, assets: string[]) => Promise<void>;
  closeBatch: () => void;
  setBatch: (patch: Partial<Pick<BatchDraft, 'provider' | 'service'>>) => void;
  setItemVariant: (asset: string, variant: PresetVariant) => void;
  /** This batch's copy of one preset; the saved presets stay. */
  setBatchPreset: (
    variant: PresetVariant,
    preset: { tripo?: TripoPreset; meshy?: MeshyPreset }
  ) => void;
  /**
   * One asset's own settings: `preset` is what the form shows, and only
   * its differences from the batch preset are kept. Null clears them.
   */
  setItemPreset: (
    asset: string,
    preset: { tripo?: TripoPreset; meshy?: MeshyPreset } | null
  ) => void;
  /** Starts the batch's presets over from the saved ones. */
  resetBatchPresets: () => void;
  /** Takes one image out of an asset's send, a ref warning's Leave out. */
  leaveOutImage: (asset: string, file: string) => void;
  /** Sends the batch; an API batch first asks for a confirm. */
  sendBatch: () => Promise<void>;
  /** The By hand checklist sheet is open. */
  checklistOpen: boolean;
  setChecklistOpen: (open: boolean) => void;
  downloads: Downloads | null;
  checkDownloads: () => Promise<void>;
  chooseMatch: (path: string, job: string | null) => void;
  /** Copies the file into the inbox for its chosen send. The original stays. */
  useDownload: (path: string) => Promise<void>;
  closeDownloads: () => void;
  /** Opens the file picker for a GLB or zip, then hands it back. */
  addModel: (id: string) => Promise<void>;
  openSite: (service: ProviderId) => Promise<void>;
  revealImage: (id: string, image: string) => Promise<void>;
  /** Opens the send panel prefilled from the earlier send `job` of `asset`. */
  openRedo: (project: string, asset: string, job: string) => Promise<void>;
  /** Opens the batch panel with each asset prefilled from its earlier send. */
  openBatchRedo: (
    project: string,
    redo: readonly { asset: string; job: string }[]
  ) => Promise<void>;
  /** One asset's Meshy texture prompt in the batch. */
  setItemPrompt: (asset: string, prompt: string) => void;
}
