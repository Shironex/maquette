/**
 * Puts the root links and images of the docs Markdown under the site's base
 * ("/docs/start/install/" becomes "/maquette/docs/start/install/"). The
 * sources keep root paths, which the Markdown twins turn into absolute URLs.
 */
import type { SatteriProcessorOptions } from '@astrojs/markdown-satteri';

type HastPlugin = NonNullable<SatteriProcessorOptions['hastPlugins']>[number];

const ATTR = { a: 'href', img: 'src' } as const;

/** `value` under `base` when it is a root path that is not there yet. */
export function underBase(base: string, value: unknown): unknown {
  const prefix = base.replace(/\/+$/, '');
  if (typeof value !== 'string' || !value.startsWith('/') || value.startsWith('//')) return value;
  if (value === prefix || value.startsWith(`${prefix}/`)) return value;
  return `${prefix}${value}`;
}

/** The Sätteri hast plugin that applies `underBase` to every link and image. */
export function baseLinks(base: string): HastPlugin {
  return {
    name: 'maquette-base-links',
    element: {
      filter: Object.keys(ATTR),
      visit(node, ctx) {
        const attr = ATTR[node.tagName as keyof typeof ATTR];
        const value = node.properties?.[attr];
        const next = underBase(base, value);
        if (next !== value) ctx.setProperty(node, attr, next);
      },
    },
  };
}
