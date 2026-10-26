import type { APIRoute, GetStaticPaths } from 'astro';
import { docsPages } from '../../lib/docs-pages';
import { toMarkdown } from '../../lib/docs-markdown';

/** Each page's Markdown twin: /docs/start/install.md, and /docs/index.md for the home. */
export const getStaticPaths = (async () =>
  (await docsPages()).map(page => ({
    params: { slug: page.slug.replace(/^docs\/?/, '') || 'index' },
    props: { markdown: toMarkdown(page) },
  }))) satisfies GetStaticPaths;

export const GET: APIRoute = ({ props }) =>
  new Response(props.markdown as string, {
    headers: { 'Content-Type': 'text/markdown; charset=utf-8' },
  });
