/**
 * The site is served under the path of SITE_URL (/maquette/), which
 * astro.config sets as `base`, in dev, preview and on GitHub Pages alike.
 * Every root path a page or the scene asks for goes through `withBase`.
 */
const BASE = import.meta.env.BASE_URL.replace(/\/+$/, '');

/** `path` ("/docs/", "/scene/models/a.glb", "/#download") under the site's base. */
export function withBase(path: string): string {
  return `${BASE}/${path.replace(/^\/+/, '')}`;
}
