/**
 * The public URL of the site, with its path. The site is served from the root
 * of its dev server and `maquette.site.json` ships into root paths, so no
 * `base` is set; links that must be absolute are built from this instead.
 */
export const SITE_URL = 'https://shironex.github.io/maquette';

/** `path` ("/docs/start/install.md") under `site`, keeping the site's own path. */
export function absoluteUrl(path: string, site: string = SITE_URL): string {
  return `${site.replace(/\/+$/, '')}/${path.replace(/^\/+/, '')}`;
}
