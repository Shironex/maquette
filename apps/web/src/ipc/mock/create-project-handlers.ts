import type {
  GameExport,
  ImportScan,
  NewProject,
  ProjectManifest,
  ProjectSummary,
  RepoInspection,
  WorkspaceConfig,
  WorkspaceSnapshot,
} from '../client';
import { clock } from '@/lib/clock';
import { DEFAULT_ROOT, HOME } from './fixtures';
import { expand, fail, slugify, type Args } from './backend-options';
import { exportAfter, merged, templateOf } from './game-templates';
import { cryptBudgets, inspectGameRepo, mazeBudgets } from './game-fixtures';
import { LEGACY_SITE } from '@/state/repo-lines';

const DEFAULT_EXPORT: GameExport = { textures: 'png', textureCap: 1024, dropMaps: false };

interface ProjectContext {
  snapshot: () => WorkspaceSnapshot;
  project: (id: string) => ProjectManifest;
  summary: (p: ProjectManifest) => ProjectSummary;
  projects: { get: () => ProjectManifest[]; set: (next: ProjectManifest[]) => void };
  config: { get: () => WorkspaceConfig; set: (next: WorkspaceConfig) => void };
}

/** The project commands and the native pickers. */
export function createProjectHandlers({
  snapshot,
  project,
  summary,
  projects,
  config,
}: ProjectContext): Record<string, (args: Args) => unknown> {
  return {
    project_create: ({ project }) => {
      snapshot();
      const next = project as NewProject;
      const name = next.name.trim();
      if (!name) fail('invalid', 'Give the project a name.');
      let id = slugify(name);
      while (projects.get().some(p => p.id === id)) id = `${id}-2`;
      const game = next.target === 'game' ? next.game : null;
      const base = game?.format === 'obj' ? mazeBudgets() : cryptBudgets();
      const template = game?.template ? templateOf(game.template) : null;
      const manifest: ProjectManifest = {
        version: 1,
        id,
        name,
        template: next.template,
        target: next.target ?? 'website',
        game: game
          ? {
              format: game.format,
              modelsDir: game.modelsDir,
              export: template ? exportAfter(DEFAULT_EXPORT, template) : DEFAULT_EXPORT,
              bake: false,
              budgets: template ? merged(base, template.budgets) : base,
              models: {},
            }
          : null,
        siteRepo: next.siteRepo ? expand(next.siteRepo) : null,
        sourceFolder: next.sourceFolder ? expand(next.sourceFolder) : null,
        createdAt: clock.now().toISOString(),
        assets: [],
      };
      projects.set([...projects.get(), manifest]);
      config.set({ ...config.get(), currentProject: id });
      return snapshot();
    },
    project_list: () => {
      snapshot();
      return projects.get().map(summary);
    },
    project_switch: ({ id }) => {
      snapshot();
      project(String(id));
      config.set({ ...config.get(), currentProject: String(id) });
      return snapshot();
    },
    project_update: ({ id, name, siteRepo }) => {
      const current = projects.get().find(p => p.id === id);
      if (!current) fail('not-found', `Could not find the project ${String(id)}.`);
      const updated: ProjectManifest = {
        ...current,
        name: String(name).trim() || current.name,
        siteRepo: siteRepo ? expand(String(siteRepo)) : null,
      };
      if (updated.siteRepo) {
        updated.siteRepoName = updated.siteRepo.split(/[\\/]/).filter(Boolean).pop() ?? null;
        updated.siteRepoState = 'here';
      } else if (current.siteRepoState === 'here') {
        updated.siteRepoState = 'none';
      }
      projects.set(projects.get().map(p => (p.id === id ? updated : p)));
      return updated;
    },
    // The real command starts the site's dev server; the mock only answers.
    site_preview: ({ project: id }) => {
      if (!project(String(id)).siteRepo) {
        fail('invalid', 'This project has no site repo yet. Pick one in Settings > Project.');
      }
      return 'http://localhost:15292/';
    },
    project_inspect_repo: ({ path }): RepoInspection => {
      const full = expand(String(path));
      const known = /site|landing|web/i.test(full);
      const game = inspectGameRepo(full).cmake;
      return {
        path: full,
        exists: known || game,
        isGit: known || game,
        hasPublicDir: known,
        site: LEGACY_SITE,
        looksLikeGame: game,
      };
    },
    project_scan_import: ({ path }): ImportScan => ({
      path: expand(String(path)),
      blendFiles: [
        { file: 'scenes/veil_platform_v3.blend', stem: 'veil_platform', version: 3 },
        { file: 'scenes/veil_platform_v2.blend', stem: 'veil_platform', version: 2 },
      ],
      toolScripts: ['bake.py', 'export.py'],
    }),
    dialog_pick_folder: ({ start }) => (typeof start === 'string' && start ? start : DEFAULT_ROOT),
    dialog_pick_file: () => '/usr/local/bin/toktx',
    dialog_pick_model: () => `${HOME}/Downloads/lantern_post.glb`,
    dialog_pick_model_file: () => `${HOME}/Desktop/lantern_post_fixed.fbx`,
  };
}
