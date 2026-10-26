import { getCollection } from 'astro:content';
import { lastUpdated, versionLine } from './docs-meta';
import { sortPages, type DocPage } from './docs-markdown';

/** Every published docs page in reading order, ready for the Markdown outputs. */
export async function docsPages(): Promise<DocPage[]> {
  const entries = await getCollection(
    'docs',
    entry => !entry.data.draft && entry.id.startsWith('docs')
  );
  return sortPages(
    entries.map(entry => ({
      slug: entry.id.replace(/\/index$/, ''),
      title: entry.data.title,
      description: entry.data.description,
      body: entry.body ?? '',
      versionLine: versionLine(lastUpdated(entry.filePath)),
    }))
  );
}
