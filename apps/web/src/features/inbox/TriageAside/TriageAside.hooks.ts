import { useState } from 'react';

/** Lowercased tags from a comma separated line, without blanks or repeats. */
export function parseTags(line: string): string[] {
  const tags = line
    .split(',')
    .map(t => t.trim().toLowerCase())
    .filter(Boolean);
  return [...new Set(tags)];
}

/** The "Add to library after" choice and its tags line. */
export function useLibraryAfter() {
  const [library, setLibrary] = useState(false);
  const [tags, setTags] = useState('');
  return { library, setLibrary, tags, setTags, after: { library, tags: parseTags(tags) } };
}
