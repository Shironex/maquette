import { describe, expect, it } from 'vitest';
import {
  absolutizeLinks,
  sortPages,
  stripMdx,
  toLlmsFull,
  toLlmsTxt,
  toMarkdown,
  type DocPage,
} from './docs-markdown';

const SITE = 'https://example.test/maquette';

const page = (slug: string, title: string, body = 'Text.'): DocPage => ({
  slug,
  title,
  description: `About ${title}.`,
  body,
  versionLine: 'Maquette 1.0.0',
});

describe('stripMdx', () => {
  it('drops import and export lines', () => {
    const md = stripMdx("import Costs from '../Costs.astro';\nexport const x = 1;\n\nText.\n");
    expect(md).toBe('Text.\n');
  });

  it('turns the Costs component into a quoted line', () => {
    const md = stripMdx('<Costs>\nThis spends credits.\nApprove it first.\n</Costs>\n');
    expect(md).toBe('> **Costs credits.**\n> This spends credits.\n> Approve it first.\n');
  });

  it('turns an inline Costs into bold text', () => {
    expect(stripMdx('<Costs>Spends credits.</Costs>\n')).toBe(
      '**Costs credits.** Spends credits.\n'
    );
  });

  it('turns a Costs credits aside the same way', () => {
    const md = stripMdx(':::caution[Costs credits]\nSpends credits.\n:::\n');
    expect(md).toBe('> **Costs credits.**\n> Spends credits.\n');
  });

  it('turns asides into quoted lines that start with their kind', () => {
    const md = stripMdx(':::note[Heads up]\nBody.\n:::\n\n:::caution\nCareful.\n:::\n');
    expect(md).toBe('> **Note: Heads up.**\n> Body.\n\n> **Caution.**\n> Careful.\n');
  });

  it('unwraps wrapper components and labels tabs', () => {
    const md = stripMdx(
      '<Tabs>\n<TabItem label="macOS">\nmac text\n</TabItem>\n<TabItem label="Windows">\nwin text\n</TabItem>\n</Tabs>\n'
    );
    expect(md).toBe('**macOS**\nmac text\n**Windows**\nwin text\n');
  });

  it('keeps the list a wrapper component holds', () => {
    const md = stripMdx('<StartCards>\n\n- [Install](/docs/start/install/)\n\n</StartCards>\n');
    expect(md).toBe('- [Install](/docs/start/install/)\n');
  });

  it('leaves fenced code as it is', () => {
    const code = '```tsx\nimport A from "a";\n<Costs>\n:::note\n```\n';
    expect(stripMdx(code)).toBe(code);
  });
});

describe('absolutizeLinks', () => {
  it('points docs pages at their Markdown twin and keeps the fragment', () => {
    const md = absolutizeLinks('[a](/docs/pipeline/compress/#settings) [home](/docs/)', SITE);
    expect(md).toBe(
      `[a](${SITE}/docs/pipeline/compress.md#settings) [home](${SITE}/docs/index.md)`
    );
  });

  it('makes other root links absolute and leaves external ones', () => {
    const md = absolutizeLinks('[t](/llms.txt) [x](https://other.test/a)', SITE);
    expect(md).toBe(`[t](${SITE}/llms.txt) [x](https://other.test/a)`);
  });

  it('does not touch code', () => {
    expect(absolutizeLinks('```\n[a](/docs/)\n```', SITE)).toBe('```\n[a](/docs/)\n```');
  });
});

describe('toMarkdown', () => {
  it('starts with the title, the lead and the version line', () => {
    const md = toMarkdown(page('docs/start/install', 'Install'), SITE);
    expect(md).toBe('# Install\n\n> About Install.\n\nMaquette 1.0.0\n\nText.\n');
  });
});

describe('the llms files', () => {
  const pages = sortPages([
    page('docs/agents/llms', 'Agents'),
    page('docs/start/install', 'Install'),
    page('docs', 'Home'),
  ]);

  it('sorts pages in reading order', () => {
    expect(pages.map(p => p.slug)).toEqual(['docs', 'docs/start/install', 'docs/agents/llms']);
  });

  it('lists every page with an absolute .md link', () => {
    const txt = toLlmsTxt(pages, '1.0.0', SITE);
    expect(txt).toContain('Docs for Maquette 1.0.0.');
    expect(txt).toContain(`- [Install](${SITE}/docs/start/install.md): About Install.`);
    expect(txt).toContain(`- [Home](${SITE}/docs/index.md): About Home.`);
    expect(txt.indexOf('## Start')).toBeLessThan(txt.indexOf('## Agents'));
  });

  it('puts every page in the full file with its source address', () => {
    const full = toLlmsFull(pages, '1.0.0', SITE);
    expect(full).toContain(`Source: ${SITE}/docs/start/install/`);
    expect(full.match(/^---$/gm)).toHaveLength(3);
  });
});
