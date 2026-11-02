/** The Plan view store's shape, shared by the store and its sync half (`plans-sync.ts`). */
import type { Connection, EdgeChange, NodeChange } from '@xyflow/react';
import type { StoreApi } from 'zustand';
import type { NodeKind, PlanDoc, PlanSummary } from '@/ipc/client';
import type { History } from './plan-history';
import type { FlowEdge, FlowNode, NodeFields } from './plans';

/** Where the person was going when a save failed on the way out. */
export type Leave =
  { to: 'list' } | { to: 'canvas'; id: string } | { to: 'project'; project: string };

/** The canvas changed outside while the person had unsaved edits to the same nodes. */
export interface Outside {
  /** The canvas as the file holds it now. */
  doc: PlanDoc;
  /** Node and edge ids both sides changed. */
  conflicts: string[];
}

/** What an outside write did to a node. */
export type NodeMark = 'added' | 'edited';

export interface PlansStore {
  project: string | null;
  list: PlanSummary[] | null;
  listError: string | null;
  /** The open canvas as the file last held it: the base the person's edits are saved against. */
  open: PlanDoc | null;
  opening: string | null;
  openError: string | null;
  nodes: FlowNode[];
  edges: FlowEdge[];
  /** A picture's stored path to the file on this computer. */
  images: Record<string, string>;
  saving: boolean;
  saveError: string | null;
  /** The open canvas holds edits the file does not have yet. */
  dirty: boolean;
  /** A save failed on the way out; the canvas stays open until answered. */
  leaving: Leave | null;
  /** A note just made, which opens for writing. */
  fresh: string | null;
  /** Unsaved edits meet an outside change to the same nodes; autosave waits for the answer. */
  outside: Outside | null;
  /** The person's own edits, to undo and redo. */
  history: History;
  /** Nodes that just came (an agent's, or picked), for the view to bring into sight. */
  arrived: string[];
  /** Nodes an outside write added or edited since the canvas opened, for their badge. */
  marks: Record<string, NodeMark>;
  clearFresh: () => void;
  /** Marks nodes as just come, or none, for the view to bring into sight. */
  setArrived: (ids: string[]) => void;
  clearMarks: () => void;
  loadList: (project: string) => Promise<void>;
  /** Makes a canvas and opens it; its id, or null when refused. */
  create: (name: string) => Promise<string | null>;
  rename: (id: string, name: string) => Promise<boolean>;
  remove: (id: string) => Promise<boolean>;
  openCanvas: (id: string) => Promise<void>;
  /** Back to the list, saving first; false (and `leaving` set) when the save failed. */
  close: () => Promise<boolean>;
  /** After a failed save on the way out: save again and go on when it works. */
  retryLeave: () => Promise<void>;
  /** After a failed save on the way out: drop the unsaved edits and go on. */
  leaveAnyway: () => Promise<void>;
  /** After a failed save on the way out: stay on the canvas. */
  stay: () => void;
  /** Saves every edit for quitting; why it could not, or null. */
  saveForQuit: () => Promise<string | null>;
  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange<FlowEdge>[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (kind: NodeKind, at: { x: number; y: number }, fields?: NodeFields) => string;
  updateNode: (id: string, fields: NodeFields) => void;
  /** Picks a picture and adds it at `at`; false when cancelled or refused. */
  addPicture: (at: { x: number; y: number }) => Promise<boolean>;
  openLink: (node: string) => Promise<void>;
  /** Saves the open canvas now when it changed; `quiet` for a failure a dialog asks about. */
  save: (quiet?: boolean) => Promise<boolean>;
  /** Reads the open canvas's file again after it changed outside, keeping unsaved edits. */
  reload: () => Promise<void>;
  /** After an outside change: where both changed the same thing, keep the person's edit. */
  keepMine: () => void;
  /** After an outside change: where both changed the same thing, take the file's. */
  useTheirs: () => void;
  undo: () => void;
  redo: () => void;
}

export type SetPlans = StoreApi<PlansStore>['setState'];
export type GetPlans = StoreApi<PlansStore>['getState'];
