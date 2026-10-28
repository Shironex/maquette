/**
 * The docs in reading order: one list the sidebar, the prev and next links,
 * llms.txt and llms-full.txt all follow. A page is its collection id, which is
 * its URL under the site root.
 */
export interface DocsGroup {
  label: string;
  slugs: readonly string[];
}

/** The docs home. It has no sidebar entry. */
export const DOCS_HOME = 'docs';

export const DOCS_GROUPS: readonly DocsGroup[] = [
  {
    label: 'Start',
    slugs: [
      'docs/start/install',
      'docs/start/updates',
      'docs/start/first-project',
      'docs/start/website-projects',
      'docs/start/game-projects',
    ],
  },
  {
    label: 'Views',
    slugs: [
      'docs/views/board',
      'docs/views/inbox',
      'docs/views/lab',
      'docs/views/pipeline',
      'docs/views/inspector',
      'docs/views/paths',
      'docs/views/motion',
      'docs/views/timeline',
      'docs/views/page',
      'docs/views/settings',
    ],
  },
  {
    label: 'Libraries',
    slugs: ['docs/views/library', 'docs/views/animations'],
  },
  {
    label: 'Pipeline steps',
    slugs: [
      'docs/pipeline/build-scene',
      'docs/pipeline/bake-light',
      'docs/pipeline/export-glb',
      'docs/pipeline/compress',
      'docs/pipeline/capture-posters',
      'docs/pipeline/ship',
    ],
  },
  {
    label: 'Reference',
    slugs: ['docs/pipeline/pipeline-json', 'docs/views/shortcuts'],
  },
  {
    label: 'Agents',
    slugs: ['docs/agents/connect', 'docs/agents/mcp-tools', 'docs/agents/llms'],
  },
];

/** Every page in reading order, the home first. */
export const DOCS_ORDER: readonly string[] = [
  DOCS_HOME,
  ...DOCS_GROUPS.flatMap(group => group.slugs),
];

/** The group a page sits in, for its breadcrumb. */
export function groupOf(slug: string): DocsGroup | undefined {
  return DOCS_GROUPS.find(group => group.slugs.includes(slug));
}

/** Where a page lives as HTML: "docs/start/install" is "/docs/start/install/". */
export function pagePath(slug: string): string {
  return `/${slug}/`;
}

/** Where a page lives as Markdown: "/docs/start/install.md", the home "/docs/index.md". */
export function markdownPath(slug: string): string {
  return slug === DOCS_HOME ? `/${DOCS_HOME}/index.md` : `/${slug}.md`;
}
