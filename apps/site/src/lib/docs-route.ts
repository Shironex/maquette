import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { DOCS_HOME, markdownPath } from './docs-nav';
import { absoluteUrl } from './site-url';

/**
 * Starlight builds its canonical and og:url from the site's origin, which drops
 * the path of the site's URL, so they are set again from it. Every page also
 * points crawlers at its Markdown twin.
 */
export const onRequest = defineRouteMiddleware(context => {
  const { starlightRoute: route } = context.locals;
  const slug = route.entry.id.replace(/\/index$/, '');
  const canonical = absoluteUrl(context.url.pathname);
  for (const tag of route.head) {
    if (tag.tag === 'link' && tag.attrs?.rel === 'canonical') tag.attrs.href = canonical;
    if (tag.tag === 'meta' && tag.attrs?.property === 'og:url') tag.attrs.content = canonical;
  }
  if (slug.startsWith(DOCS_HOME)) {
    route.head.push({
      tag: 'link',
      attrs: { rel: 'alternate', type: 'text/markdown', href: absoluteUrl(markdownPath(slug)) },
      content: '',
    });
  }
});
