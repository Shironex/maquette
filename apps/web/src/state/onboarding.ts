/**
 * The onboarding flow as plain data and functions, so the rules about when
 * Continue works, what it says and where it goes are tested without React.
 */
import type {
  GameFormat,
  GameTemplateId,
  GameRepoInspection,
  NewProject,
  ProjectTarget,
  ProjectTemplate,
  RepoInspection,
  RootInspection,
  ToolReport,
  WorkspaceConfig,
} from '@/ipc/client';
import type { FieldTone } from '@maquette/ui';
import { formatK, shortPath } from '@/lib/format';
import { modelLabel, rigSpecLabel } from './provider-presets';
import { NO_LISTS, hasErrors, validateConfig, type ModelLists } from './settings-form';
import { MISSING_EFFECT, SHORT_NAME, summarizeTools } from './toolchain';

export const STEPS = [
  'Welcome',
  'Workspace',
  'Toolchain',
  'Defaults',
  'Agents',
  'First project',
  'Ready',
] as const;

export const STEP = {
  welcome: 0,
  workspace: 1,
  toolchain: 2,
  defaults: 3,
  agents: 4,
  project: 5,
  ready: 6,
} as const;

export const NAME_MAX = 80;

/** What the first project is called until someone names it. */
export const DEFAULT_PROJECT_NAME = 'My first project';

export type ProjectMode = 'template' | 'import';

export interface Draft {
  root: string;
  inspection: RootInspection | null;
  rootError: string | null;
  config: WorkspaceConfig;
  mode: ProjectMode;
  template: ProjectTemplate;
  name: string;
  siteRepo: string;
  sourceFolder: string;
  repo: RepoInspection | null;
  /** Where the project ships: a website, or a game repo. */
  target: ProjectTarget;
  /** What New project found in the game repo (siteRepo). */
  game: GameRepoInspection | null;
  gameFormat: GameFormat;
  modelsDir: string;
  /** A game template whose folder budgets a new game starts with. */
  gameTemplate: GameTemplateId | null;
}

export const DEFAULT_MODELS_DIR = 'assets/models';

const NO_GAME = {
  target: 'website',
  game: null,
  gameFormat: 'glb',
  modelsDir: DEFAULT_MODELS_DIR,
  gameTemplate: null,
} as const;

/** The fields of the first project step, shared with New project in the switcher. */
export type ProjectFields = Pick<
  Draft,
  | 'mode'
  | 'template'
  | 'name'
  | 'siteRepo'
  | 'sourceFolder'
  | 'repo'
  | 'target'
  | 'game'
  | 'gameFormat'
  | 'modelsDir'
  | 'gameTemplate'
>;

export function emptyProjectFields(name = ''): ProjectFields {
  return {
    mode: 'template',
    template: 'floating-diorama',
    name,
    siteRepo: '',
    sourceFolder: '',
    repo: null,
    ...NO_GAME,
  };
}

/** A name, a folder when importing, and for a game a game repo the check found. */
export function projectFieldsReady(fields: ProjectFields): boolean {
  if (nameError(fields.name) !== null) return false;
  if (fields.target === 'game') {
    return fields.game?.exists === true && fields.modelsDir.trim() !== '';
  }
  return fields.mode === 'template' || fields.sourceFolder.trim() !== '';
}

export function newProjectRequest(fields: ProjectFields): NewProject {
  if (fields.target === 'game') {
    return {
      name: fields.name.trim(),
      template: 'game-import',
      target: 'game',
      siteRepo: fields.siteRepo.trim() || null,
      sourceFolder: null,
      game: {
        format: fields.gameFormat,
        modelsDir: fields.modelsDir.trim(),
        template: fields.gameTemplate,
      },
    };
  }
  return {
    name: fields.name.trim(),
    template: fields.mode === 'import' ? 'imported' : fields.template,
    target: 'website',
    siteRepo: fields.siteRepo.trim() || null,
    sourceFolder: fields.mode === 'import' ? fields.sourceFolder.trim() : null,
  };
}

export interface FlowState {
  draft: Draft;
  tools: readonly ToolReport[];
  checking: boolean;
  /** The workspace and the first project exist on disk. */
  created: boolean;
  busy: boolean;
}

export function initialDraft(root: string, config: WorkspaceConfig): Draft {
  return {
    root,
    inspection: null,
    rootError: null,
    config,
    mode: 'template',
    template: 'floating-diorama',
    name: DEFAULT_PROJECT_NAME,
    siteRepo: '',
    sourceFolder: '',
    repo: null,
    ...NO_GAME,
  };
}

export function nameError(name: string): string | null {
  const trimmed = name.trim();
  if (!trimmed) return 'Give the project a name.';
  if ([...trimmed].length > NAME_MAX) return `Keep the name under ${NAME_MAX} characters.`;
  return null;
}

export function rootUsable(draft: Draft): boolean {
  const state = draft.inspection?.state;
  return !draft.rootError && (state === 'empty' || state === 'missing' || state === 'workspace');
}

/** Picking a folder that already holds maquette.json opens it and skips the rest. */
export function opensExisting(draft: Draft, created: boolean): boolean {
  return !created && draft.inspection?.state === 'workspace';
}

export function canContinue(step: number, flow: FlowState): boolean {
  if (flow.busy) return false;
  const { draft } = flow;
  switch (step) {
    case STEP.workspace:
      return flow.created || rootUsable(draft);
    case STEP.toolchain:
      return !flow.checking && flow.tools.length > 0 && !summarizeTools(flow.tools).blocked;
    case STEP.defaults:
    case STEP.agents:
      return !hasErrors(validateConfig(draft.config));
    case STEP.project:
      return projectFieldsReady(draft);
    default:
      return true;
  }
}

export function continueLabel(step: number, flow: FlowState): string {
  if (step === STEP.workspace && opensExisting(flow.draft, flow.created)) return 'Open workspace';
  if (step === STEP.project) return flow.created ? 'Save project' : 'Create project';
  if (step === STEP.ready) return `Open ${flow.draft.name.trim() || 'project'}`;
  if (flow.created && step > STEP.welcome) return 'Save and review';
  return 'Continue';
}

/** After creation, an edited step returns to Ready instead of walking on. */
export function nextStep(step: number, created: boolean): number {
  if (created && step >= STEP.workspace && step < STEP.ready) return STEP.ready;
  return Math.min(step + 1, STEP.ready);
}

export function rootNote(
  draft: Draft,
  created: boolean
): { tone: FieldTone; text: string } | undefined {
  if (created) return { tone: 'found', text: 'Workspace created. Move it later from Settings.' };
  if (draft.rootError) return { tone: 'missing', text: draft.rootError };
  switch (draft.inspection?.state) {
    case 'empty':
      return { tone: 'found', text: 'Empty folder, ready to use' };
    case 'missing':
      return { tone: 'found', text: 'New folder, I will create it' };
    case 'workspace':
      return { tone: 'found', text: 'Maquette workspace found. Continue opens it.' };
    case 'not-empty':
      return {
        tone: 'missing',
        text: 'This folder has other files. Pick an empty folder or an existing workspace.',
      };
    default:
      return undefined;
  }
}

export const TEMPLATE_LABEL: Record<ProjectTemplate, string> = {
  'floating-diorama': 'Floating diorama',
  room: 'Room',
  'character-showcase': 'Character showcase',
  imported: 'Imported folder',
  'game-import': 'Game models',
};

export interface ReadyRow {
  id: 'workspace' | 'toolchain' | 'defaults' | 'agents' | 'project';
  title: string;
  /** A path or address shown in mono before the text. */
  mono?: string;
  text: string;
  /** A short warning shown in amber, then what it costs. */
  warning?: string;
  effect?: string;
  step: number;
}

function upper(s: string) {
  return s.toUpperCase();
}

export function defaultsSummary(config: WorkspaceConfig, lists: ModelLists = NO_LISTS): string {
  const { providers, budgets, compression } = config;
  const services = `${tripoLabel(providers.tripo.prop.model, lists)}, ${modelLabel(
    lists.meshyModels,
    providers.meshy.prop.aiModel
  )}`;
  const tripo = providers.tripo.character;
  const character = [
    `Characters: ${tripo.faceLimit ? `${formatK(tripo.faceLimit)} faces` : 'adaptive faces'}`,
    tripo.multiView ? 'multi-view' : null,
    tripo.rig ? `rig with ${rigSpecLabel(tripo.rigSpec)}` : null,
  ]
    .filter(Boolean)
    .join(', ');
  const mb = (budgets.sceneMb ?? 0).toFixed(1);
  const web = `${mb}\u00a0MB scene, ${formatK(budgets.trisPerModel)}\u00a0tris, ${
    compression.textureSize
  }, ${upper(compression.textureCodec)}, meshopt ${compression.meshopt}`;
  return `${services}. ${character}. ${web}`;
}

function tripoLabel(model: string, lists: ModelLists): string {
  return `Tripo ${modelLabel(lists.tripoModels, model)}`;
}

export function agentsSummary(config: WorkspaceConfig): { mono?: string; text: string } {
  const { rules } = config.agents;
  const ruleText = [
    rules.blenderHeadlessOnly ? 'Headless Blender' : null,
    rules.neverCaptureScreen ? 'no screen capture' : null,
    rules.askBeforeShipping ? 'ask before shipping' : null,
  ].filter(Boolean);
  const rulesPart = ruleText.length ? ruleText.join(', ') : 'No agent rules';
  const sentence = rulesPart.charAt(0).toUpperCase() + rulesPart.slice(1);
  if (!config.agents.mcpEnabled) return { text: `MCP server off. ${sentence}` };
  return {
    mono: `127.0.0.1:${config.agents.mcpPort}`,
    text: `MCP server on this port once it ships. ${sentence}`,
  };
}

export function readySummary(
  draft: Draft,
  tools: readonly ToolReport[],
  lists: ModelLists = NO_LISTS
): ReadyRow[] {
  const summary = summarizeTools(tools);
  const optionalMissing = summary.missing.filter(id => !tools.find(t => t.id === id)?.required);
  const { layout } = draft.config;
  const agents = agentsSummary(draft.config);
  const project = draft.name.trim();
  return [
    {
      id: 'workspace',
      title: 'Workspace',
      mono: draft.inspection?.display ?? draft.root,
      text: `with ${layout.projects}/, ${layout.inbox}/ and ${layout.cache}/`,
      step: STEP.workspace,
    },
    {
      id: 'toolchain',
      title: 'Toolchain',
      text: `${summary.found} of ${summary.total} found.`,
      warning: optionalMissing.length
        ? `${optionalMissing.map(id => SHORT_NAME[id]).join(', ')} missing`
        : undefined,
      effect: missingEffect(tools),
      step: STEP.toolchain,
    },
    {
      id: 'defaults',
      title: 'Defaults',
      text: defaultsSummary(draft.config, lists),
      step: STEP.defaults,
    },
    { id: 'agents', title: 'Agents', ...agents, step: STEP.agents },
    {
      id: 'project',
      title: 'First project',
      text:
        draft.target === 'game'
          ? `${project}, a game, ships to ${shortPath(draft.siteRepo.trim(), 2)}`
          : `${project}, ${TEMPLATE_LABEL[draft.mode === 'import' ? 'imported' : draft.template]}${
              draft.siteRepo.trim()
                ? `, ships to ${shortPath(draft.siteRepo.trim(), 2)}`
                : ', no site repo yet'
            }`,
      step: STEP.project,
    },
  ];
}

/** The tail after the amber warning on the Toolchain row. */
export function missingEffect(tools: readonly ToolReport[]): string | undefined {
  const optional = summarizeTools(tools).missing.filter(
    id => !tools.find(t => t.id === id)?.required
  );
  if (optional.length !== 1 || !optional[0])
    return optional.length ? 'some steps wait until they are installed' : undefined;
  return MISSING_EFFECT[optional[0]];
}
