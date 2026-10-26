import type { APIRoute } from 'astro';
import { APP_VERSION } from '../lib/docs-meta';
import { docsPages } from '../lib/docs-pages';
import { toLlmsTxt } from '../lib/docs-markdown';

export const GET: APIRoute = async () =>
  new Response(toLlmsTxt(await docsPages(), APP_VERSION), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
