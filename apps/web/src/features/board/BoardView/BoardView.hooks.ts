import { useEffect, useRef, useState, type MouseEvent } from 'react';
import {
  commands,
  errorMessage,
  type Asset,
  type ModelFile,
  type ProjectManifest,
  type Stage,
} from '@/ipc/client';
import { useNativeDrop } from '@/lib/useNativeDrop';
import { columns } from '@/state/board';
import { filterRows, listRow, sortRows } from '@/state/board-list';
import { MAX_BATCH, nextSelection } from '@/state/batch';
import { STAGES } from '@/state/pipeline';
import { useAppStore } from '@/state/use-app-store';
import { prefsOf, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { useProvidersStore } from '@/state/use-providers-store';

const MODEL_FILE = /\.(glb|zip)$/i;

/** The By hand job of the card under a point, for native GLB drops. */
function jobAt(x: number, y: number): string | null {
  const card = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop-job]');
  return card?.dataset.dropJob ?? null;
}

const NO_MODELS: readonly ModelFile[] = [];

/**
 * The open project, its assets by column or as list rows (filtered and
 * sorted as remembered), and the order the cards or rows read in.
 */
export function useBoardData() {
  const workspace = useAppStore(s => s.workspace);
  const jobs = useProvidersStore(s => s.jobs);
  const byProject = useBoardPrefsStore(s => s.byProject);
  const project = workspace?.project;
  const models = usePipelineStore(s =>
    s.view && s.view.project === project?.id ? s.view.models : NO_MODELS
  );
  const assets = project?.assets ?? [];
  const otherProjects = (workspace?.projects ?? []).filter(p => p.id !== project?.id);
  const prefs = prefsOf(byProject, project?.id);
  const byStage = columns(assets);
  const rows = sortRows(
    filterRows(
      assets.map(a => listRow(a, models)),
      prefs.stage,
      prefs.query
    ),
    prefs.sort
  );
  const order =
    prefs.layout === 'list'
      ? rows.map(r => r.asset.id)
      : STAGES.flatMap(stage => byStage[stage].map(a => a.id));
  const budget = workspace?.config.budgets.sceneMb ?? 3;
  return { workspace, project, assets, jobs, otherProjects, prefs, byStage, rows, order, budget };
}

/** The cards picked for one batch, with Shift ranges, Escape and the send. */
export function useBoardSelection(projectId: string | undefined, order: readonly string[]) {
  const openBatch = useProvidersStore(s => s.openBatch);
  const batchesSent = useProvidersStore(s => s.batchesSent);
  const [selected, setSelected] = useState<string[]>([]);
  const [anchor, setAnchor] = useState<string | null>(null);
  const present = selected.filter(id => order.includes(id));

  // A selection belongs to the project it was made in.
  useEffect(() => {
    setSelected([]);
    setAnchor(null);
  }, [projectId]);
  // A confirmed Send to 3D hands the selection over, so the cards let go.
  const lastSent = useRef(batchesSent);
  useEffect(() => {
    if (lastSent.current === batchesSent) return;
    lastSent.current = batchesSent;
    setSelected([]);
    setAnchor(null);
  }, [batchesSent]);
  useEffect(() => {
    if (selected.length === 0) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !document.querySelector('[aria-modal="true"]')) setSelected([]);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selected.length]);

  const check = (id: string, on: boolean, range: boolean) => {
    setSelected(s => nextSelection(s, order, id, on, range, anchor));
    setAnchor(id);
  };
  /** Shift and a click anywhere on a card selects it, and the cards between. */
  const shiftClick = (id: string) => (e: MouseEvent) => {
    if (!e.shiftKey) return;
    const target = e.target as HTMLElement;
    if (target.closest('[aria-haspopup], [role="menu"], [role="checkbox"], [data-card-action]'))
      return;
    e.preventDefault();
    check(id, !present.includes(id), true);
  };
  const send = () => {
    if (!projectId || present.length === 0) return;
    void openBatch(projectId, present.slice(0, MAX_BATCH));
  };
  const clear = () => setSelected([]);
  return { present, check, shiftClick, send, clear };
}

/** A card's menu, its copy and rename, and moving cards between columns. */
export function useCardActions(project: ProjectManifest | null | undefined) {
  const setProject = useAppStore(s => s.setProject);
  const notify = useAppStore(s => s.notify);
  const loadProjects = useProjectsStore(s => s.load);
  const [menuFor, setMenuFor] = useState<string | null>(null);
  /** The card whose menu shows the projects to copy it to. */
  const [copyFor, setCopyFor] = useState<string | null>(null);
  const [copying, setCopying] = useState(false);
  const [renaming, setRenaming] = useState<Asset | null>(null);
  /** The card whose real size and fix-ups are open. */
  const [sizing, setSizing] = useState<Asset | null>(null);

  // An open menu or dialog belongs to the project it was opened in.
  useEffect(() => {
    setRenaming(null);
    setSizing(null);
    setMenuFor(null);
    setCopyFor(null);
  }, [project?.id]);

  const later = (what: string) =>
    notify({
      status: 'info',
      title: `${what} arrives in a later version`,
      detail: 'This build runs the pipeline and checks models in the inbox.',
    });

  const move = async (asset: Asset, stage: Stage) => {
    if (!project || asset.stage === stage) return;
    try {
      setProject(await commands.assetMove(project.id, asset.id, stage));
    } catch (error) {
      notify({
        status: 'failed',
        title: `Could not move ${asset.name}`,
        detail: errorMessage(error),
      });
    }
  };
  /** A card dragged onto a column. */
  const dropOn = (stage: Stage, id: string) => {
    const asset = (project?.assets ?? []).find(a => a.id === id);
    if (asset) void move(asset, stage);
  };
  const copyTo = async (asset: Asset, to: { id: string; name: string }) => {
    if (!project || copying) return;
    setCopying(true);
    try {
      const done = await commands.assetCopyToProject(project.id, asset.id, to.id);
      const versions = done.asset.versions.length;
      const what = [
        done.refs.length > 0
          ? `${done.refs.length} ${done.refs.length === 1 ? 'ref' : 'refs'}`
          : null,
        versions > 0 ? `${versions} ${versions === 1 ? 'version' : 'versions'}` : null,
      ].filter(Boolean);
      notify({
        status: 'done',
        title: `Copied ${asset.name} to ${to.name}`,
        detail: `${done.asset.name} is on its Board${what.length ? ` with ${what.join(' and ')}` : ''}. The card stays here too.`,
      });
      void loadProjects();
    } catch (error) {
      notify({
        status: 'failed',
        title: `Could not copy ${asset.name}`,
        detail: errorMessage(error),
      });
    } finally {
      setCopying(false);
      setMenuFor(null);
      setCopyFor(null);
    }
  };
  const openInBlender = async (asset: Asset) => {
    if (!project) return;
    try {
      await commands.assetOpenInBlender(project.id, asset.id);
    } catch (error) {
      notify({ status: 'failed', title: 'Could not open Blender', detail: errorMessage(error) });
    }
  };
  const renamed = (next: ProjectManifest) => {
    setProject(next);
    setRenaming(null);
  };

  return {
    menuFor,
    setMenuFor,
    copyFor,
    setCopyFor,
    copying,
    renaming,
    setRenaming,
    renamed,
    sizing,
    setSizing,
    later,
    move,
    dropOn,
    copyTo,
    openInBlender,
  };
}

/** The By hand card under a native GLB drag, and the hand back on drop. */
export function useCardDrop() {
  const notify = useAppStore(s => s.notify);
  const handBack = useProvidersStore(s => s.handBack);
  const [dropJob, setDropJob] = useState<string | null>(null);
  useNativeDrop({
    onOver: ({ x, y }) => setDropJob(jobAt(x, y)),
    onLeave: () => setDropJob(null),
    onDrop: ({ x, y }, paths) => {
      setDropJob(null);
      const job = jobAt(x, y);
      const model = paths.find(p => MODEL_FILE.test(p));
      if (job && model) void handBack(job, model);
      else if (job)
        notify({
          status: 'info',
          title: 'Drop a .glb or .zip file',
          detail: 'The card takes the GLB you made by hand, or the zip the web app downloads.',
        });
    },
  });
  return dropJob;
}
