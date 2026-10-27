import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { DOCS_HOME, markdownPath } from './docs-nav';
import { absoluteUrl } from './site-url';

/**
 * Every docs page points crawlers at its Markdown twin. Starlight's canonical
 * and og:url already carry the site's path, since it is the base.
 */
export const onRequest = defineRouteMiddleware(context => {
  const { starlightRoute: route } = context.locals;
  const slug = route.entry.id.replace(/\/index$/, '');
  if (slug.startsWith(DOCS_HOME)) {
    route.head.push({
      tag: 'link',
      attrs: { rel: 'alternate', type: 'text/markdown', href: absoluteUrl(markdownPath(slug)) },
      content: '',
    });
  }
});
