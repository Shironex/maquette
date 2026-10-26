import { DOCS_GROUPS, DOCS_HOME, DOCS_ORDER, markdownPath, pagePath } from './docs-nav';
import { absoluteUrl } from './site-url';

/** What a docs entry gives the Markdown outputs. */
export interface DocPage {
  slug: string;
  title: string;
  description: string;
  /** The source of the page, frontmatter already removed. */
  body: string;
  /** "Maquette 0.1.0 · Last updated 26 Oct 2026". */
  versionLine: string;
}

const FENCE = /^\s*(```|~~~)/;
const IMPORT_EXPORT = /^(import|export)\s.*$/;
const ASIDE_OPEN = /^:::(note|tip|caution|danger)(?:\[(.*)\])?\s*$/;
const ASIDE_CLOSE = /^:::\s*$/;
/** A component tag on a line of its own, such as `<Steps>` or `<TabItem label="macOS">`. */
const TAG_LINE = /^\s*<\/?([A-Z][A-Za-z]*)\b([^>]*)>\s*$/;
const COSTS_INLINE = /<Costs>([\s\S]*?)<\/Costs>/g;
const ASIDE_WORD = { note: 'Note', tip: 'Tip', caution: 'Caution', danger: 'Danger' } as const;

const quote = (lines: string[]) => lines.map(line => (line ? `> ${line}` : '>')).join('\n');

/**
 * The page as plain Markdown. MDX imports and component tags go: a component
 * only wraps prose, so unwrapping it loses nothing. Callouts become quoted
 * lines that start with Note, Tip, Caution or Costs credits. Fenced code is
 * left as it is.
 */
export function stripMdx(source: string): string {
  const out: string[] = [];
  let inFence = false;
  let aside: { head: string; lines: string[] } | undefined;
  let costs: string[] | undefined;

  const flushAside = () => {
    if (!aside) return;
    out.push(quote([aside.head, ...aside.lines]));
    aside = undefined;
  };

  for (const line of source.replace(/\r\n/g, '\n').split('\n')) {
    if (FENCE.test(line)) inFence = !inFence;
    if (inFence || FENCE.test(line)) {
      out.push(line);
      continue;
    }
    if (IMPORT_EXPORT.test(line)) continue;

    const open = ASIDE_OPEN.exec(line);
    if (open) {
      const word = ASIDE_WORD[open[1] as keyof typeof ASIDE_WORD];
      const title = open[2]?.trim();
      const head =
        title === 'Costs credits'
          ? '**Costs credits.**'
          : `**${word}${title ? `: ${title}` : ''}.**`;
      aside = { head, lines: [] };
      continue;
    }
    if (aside) {
      if (ASIDE_CLOSE.test(line)) flushAside();
      else aside.lines.push(line);
      continue;
    }

    const tag = TAG_LINE.exec(line);
    if (tag) {
      const [, name = '', attrs = ''] = tag;
      const closing = line.trimStart().startsWith('</');
      if (name === 'Costs') {
        if (closing) {
          if (costs) out.push(quote(['**Costs credits.**', ...costs]));
          costs = undefined;
        } else {
          costs = [];
        }
      } else if (name === 'TabItem' && !closing) {
        const label = /label="([^"]*)"/.exec(attrs)?.[1];
        if (label) out.push(`**${label}**`);
      }
      continue;
    }
    if (costs) {
      costs.push(line);
      continue;
    }
    out.push(line.replace(COSTS_INLINE, (_, text: string) => `**Costs credits.** ${text.trim()}`));
  }
  flushAside();
  return `${out
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim()}\n`;
}

/**
 * Root-relative links become absolute ones, since a Markdown file read on its
 * own has no site around it. A docs page points at its Markdown twin.
 */
export function absolutizeLinks(markdown: string, site?: string): string {
  let inFence = false;
  return markdown
    .split('\n')
    .map(line => {
      if (FENCE.test(line)) inFence = !inFence;
      if (inFence || FENCE.test(line)) return line;
      return line.replace(/\]\((\/[^)\s]*)\)/g, (_, href: string) => {
        const [path = '', fragment] = href.split('#');
        const docs = path.startsWith(`/${DOCS_HOME}/`) && path.endsWith('/');
        const target = docs ? markdownPath(path.slice(1, -1)) : path;
        return `](${absoluteUrl(target, site)}${fragment ? `#${fragment}` : ''})`;
      });
    })
    .join('\n');
}

/** One page as the Markdown twin: title, lead, version line, then the body. */
export function toMarkdown(page: DocPage, site?: string): string {
  const body = absolutizeLinks(stripMdx(page.body), site);
  return [`# ${page.title}`, `> ${page.description}`, page.versionLine, body].join('\n\n');
}

/** Pages in reading order. Pages the nav does not list come last, by slug. */
export function sortPages<T extends { slug: string }>(pages: readonly T[]): T[] {
  const rank = (slug: string) => {
    const index = DOCS_ORDER.indexOf(slug);
    return index === -1 ? DOCS_ORDER.length : index;
  };
  return [...pages].sort((a, b) => rank(a.slug) - rank(b.slug) || a.slug.localeCompare(b.slug));
}

const SUMMARY = 'A desktop studio that turns AI concept art into real-time 3D web scenes.';

/** /llms.txt: a title, one line, then a link per page grouped like the sidebar. */
export function toLlmsTxt(pages: readonly DocPage[], appVersion: string, site?: string): string {
  const link = (page: DocPage) =>
    `- [${page.title}](${absoluteUrl(markdownPath(page.slug), site)}): ${page.description}`;
  const home = pages.find(page => page.slug === DOCS_HOME);
  const sections = DOCS_GROUPS.map(group => {
    const members = pages.filter(page => group.slugs.includes(page.slug));
    return members.length > 0 ? `## ${group.label}\n\n${members.map(link).join('\n')}` : '';
  }).filter(Boolean);
  return (
    [
      '# Maquette',
      `> ${SUMMARY} Docs for Maquette ${appVersion}.`,
      ...(home ? [`## Overview\n\n${link(home)}`] : []),
      ...sections,
    ].join('\n\n') + '\n'
  );
}

/** /llms-full.txt: every page, one after the other, each with its source address. */
export function toLlmsFull(pages: readonly DocPage[], appVersion: string, site?: string): string {
  const chunks = pages.map(
    page => `Source: ${absoluteUrl(pagePath(page.slug), site)}\n\n${toMarkdown(page, site)}`
  );
  return `# Maquette\n\n> ${SUMMARY} Docs for Maquette ${appVersion}.\n\n---\n\n${chunks.join('\n---\n\n')}`;
}
