import type { APIRoute } from 'astro';
import { APP_VERSION } from '../lib/docs-meta';
import { docsPages } from '../lib/docs-pages';
import { toLlmsFull } from '../lib/docs-markdown';

export const GET: APIRoute = async () =>
  new Response(toLlmsFull(await docsPages(), APP_VERSION), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
