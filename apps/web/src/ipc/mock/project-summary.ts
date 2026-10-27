/** A project's row in the project switcher, as the mock backend lists it. */
import type { ProjectManifest, ProjectSummary, ProviderJob } from '../client';
import { DEFAULT_ROOT } from './fixtures';
import { expand } from './backend-options';

export interface SummaryDeps {
  activeRun: (id: string) => string | null;
  /** Concept jobs still running for the project. */
  concepts: (id: string) => number;
  jobs: () => ProviderJob[] | null | undefined;
}

/** Its newest scene picture (else its newest picture) and what is running for it. */
export function projectSummary(p: ProjectManifest, deps: SummaryDeps): ProjectSummary {
  const dir = `${expand(DEFAULT_ROOT)}/projects/${p.id}`;
  const pictured = (p.assets ?? []).filter(a => a.thumbnail);
  const thumb =
    pictured.find(a => a.kind === 'scene')?.thumbnail ??
    [...pictured].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))[0]?.thumbnail ??
    null;
  const sends = (deps.jobs() ?? []).filter(
    j => j.project === p.id && ['approval', 'queued', 'running', 'waiting'].includes(j.state)
  ).length;
  return {
    id: p.id,
    name: p.name,
    template: p.template,
    target: p.target ?? 'website',
    siteRepo: p.siteRepo,
    siteRepoName: p.siteRepoName ?? null,
    siteRepoState: p.siteRepoState ?? (p.siteRepo ? 'here' : 'none'),
    createdAt: p.createdAt,
    assetCount: (p.assets ?? []).length,
    dir,
    thumb,
    activity: { run: deps.activeRun(p.id), concepts: deps.concepts(p.id), sends },
  };
}
