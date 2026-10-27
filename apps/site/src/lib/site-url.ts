/**
 * The public URL of the site, with its path. astro.config sets that path
 * (/maquette/) as `base`; links that must be absolute are built from this.
 */
export const SITE_URL = 'https://shironex.github.io/maquette';

/** `path` ("/docs/start/install.md") under `site`, keeping the site's own path. */
export function absoluteUrl(path: string, site: string = SITE_URL): string {
  return `${site.replace(/\/+$/, '')}/${path.replace(/^\/+/, '')}`;
}
