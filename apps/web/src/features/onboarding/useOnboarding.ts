/**
 * Drives onboarding: the draft, the step, and the calls that create the
 * workspace and the first project. Rules about the steps live in
 * `state/onboarding.ts`; this hook only sequences them.
 */
import { useCallback, useEffect, useState } from 'react';
import { commands, errorMessage, type WorkspaceSnapshot } from '@/ipc/client';
import { useAppStore } from '@/state/app-store';
import {
  STEP,
  canContinue,
  continueLabel,
  initialDraft,
  nextStep,
  opensExisting,
  type Draft,
  type FlowState,
} from '@/state/onboarding';

export function useOnboarding() {
  const boot = useAppStore(s => s.boot);
  const tools = useAppStore(s => s.tools);
  const detectTools = useAppStore(s => s.detectTools);
  const enterApp = useAppStore(s => s.enterApp);
  const notify = useAppStore(s => s.notify);

  const [step, setStep] = useState<number>(STEP.welcome);
  const [draft, setDraft] = useState<Draft>(() =>
    initialDraft(boot?.defaultRoot ?? '~/Maquette', boot!.defaultConfig)
  );
  const [created, setCreated] = useState(false);
  const [busy, setBusy] = useState(false);
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);

  const update = useCallback((patch: Partial<Draft>) => setDraft(d => ({ ...d, ...patch })), []);

  const fail = useCallback(
    (title: string, error: unknown) =>
      notify({ status: 'failed', title, detail: errorMessage(error) }),
    [notify]
  );

  const inspectRoot = useCallback(
    async (root: string) => {
      update({ root, inspection: null, rootError: null });
      if (!root.trim()) {
        update({ rootError: 'Pick a folder for the workspace.' });
        return;
      }
      try {
        const inspection = await commands.workspaceInspect(root);
        setDraft(d => (d.root === root ? { ...d, inspection, rootError: null } : d));
      } catch (error) {
        setDraft(d => (d.root === root ? { ...d, rootError: errorMessage(error) } : d));
      }
    },
    [update]
  );

  // The steps that need data fetch it once when they first show.
  useEffect(() => {
    if (step === STEP.workspace && !draft.inspection && !draft.rootError && !created) {
      void inspectRoot(draft.root);
    }
    if (step === STEP.toolchain && tools.reports.length === 0 && !tools.checking) {
      void detectTools();
    }
  }, [
    step,
    draft.inspection,
    draft.rootError,
    draft.root,
    created,
    inspectRoot,
    tools,
    detectTools,
  ]);

  const flow: FlowState = {
    draft,
    tools: tools.reports,
    checking: tools.checking,
    created,
    busy,
  };

  const pickFolder = useCallback(
    async (title: string, start?: string) => {
      try {
        return await commands.dialogPickFolder(title, start?.trim() ? start : null);
      } catch (error) {
        fail('Could not open the folder picker', error);
        return null;
      }
    },
    [fail]
  );

  const browseRoot = async () => {
    const picked = await pickFolder('Pick a workspace folder', draft.root);
    if (picked) await inspectRoot(picked);
  };

  const checkRepo = async (path: string) => {
    update({ siteRepo: path, repo: null });
    if (!path.trim()) return;
    try {
      update({ repo: await commands.projectInspectRepo(path) });
    } catch (error) {
      fail('Could not check that folder', error);
    }
  };

  const browseRepo = async () => {
    const picked = await pickFolder('Pick the site repo', draft.siteRepo);
    if (picked) await checkRepo(picked);
  };

  const browseSource = async () => {
    const picked = await pickFolder('Pick the folder to import', draft.sourceFolder);
    if (!picked) return;
    update({ sourceFolder: picked });
  };

  const openWorkspace = async (root: string) => {
    setBusy(true);
    try {
      enterApp(await commands.workspaceOpen(root));
    } catch (error) {
      fail('Could not open that workspace', error);
    } finally {
      setBusy(false);
    }
  };

  const openExistingFromWelcome = async () => {
    const picked = await pickFolder('Open a Maquette workspace', draft.root);
    if (picked) await openWorkspace(picked);
  };

  async function createEverything(): Promise<boolean> {
    const root = draft.inspection?.path ?? draft.root;
    await commands.workspaceCreate(root, draft.config);
    const snap = await commands.projectCreate({
      name: draft.name.trim(),
      template: draft.mode === 'import' ? 'imported' : draft.template,
      siteRepo: draft.siteRepo.trim() || null,
      sourceFolder: draft.mode === 'import' ? draft.sourceFolder.trim() : null,
    });
    setSnapshot(snap);
    setCreated(true);
    return true;
  }

  async function saveEdits(): Promise<void> {
    if (!snapshot) return;
    if (step === STEP.defaults || step === STEP.agents) {
      const config = await commands.workspaceSaveConfig({
        ...draft.config,
        currentProject: snapshot.config.currentProject,
      });
      setSnapshot({ ...snapshot, config });
    }
    if (step === STEP.project && snapshot.project) {
      const project = await commands.projectUpdate(
        snapshot.project.id,
        draft.name.trim(),
        draft.siteRepo.trim() || null
      );
      setSnapshot({
        ...snapshot,
        project,
        projects: snapshot.projects.map(p => (p.id === project.id ? project : p)),
      });
    }
  }

  const next = async () => {
    if (!canContinue(step, flow)) return;
    if (step === STEP.workspace && opensExisting(draft, created)) {
      await openWorkspace(draft.inspection?.path ?? draft.root);
      return;
    }
    if (step === STEP.ready) {
      if (snapshot) enterApp(snapshot);
      return;
    }
    setBusy(true);
    try {
      let nowCreated = created;
      if (step === STEP.project && !created) nowCreated = await createEverything();
      else if (created) await saveEdits();
      setStep(nextStep(step, nowCreated));
    } catch (error) {
      fail(
        step === STEP.project && !created ? 'Could not create the project' : 'Could not save',
        error
      );
    } finally {
      setBusy(false);
    }
  };

  const back = () => setStep(s => Math.max(STEP.welcome, s - 1));

  return {
    step,
    setStep,
    draft,
    update,
    flow,
    created,
    snapshot,
    canContinue: canContinue(step, flow),
    continueLabel: continueLabel(step, flow),
    next,
    back,
    inspectRoot,
    browseRoot,
    checkRepo,
    browseRepo,
    browseSource,
    openExistingFromWelcome,
  };
}

export type OnboardingApi = ReturnType<typeof useOnboarding>;
