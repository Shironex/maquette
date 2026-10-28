import { defineConfig } from 'astro/config';
import { satteri } from '@astrojs/markdown-satteri';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { basisTranscoder } from './vite/basis-transcoder.ts';
import { DOCS_GROUPS } from './src/lib/docs-nav.ts';
import { baseLinks } from './src/lib/base-links.ts';
import { SITE_URL } from './src/lib/site-url.ts';
import { platformScript } from './src/lib/platform.ts';

// Next to the web app's 15190 and its worktree ports; preview uses the same.
const PORT = 15292;

// GitHub Pages serves the site under the path of SITE_URL (/maquette/), so
// that path is the base in dev and preview too. The trailing slash keeps the
// sitemap from listing the home page twice.
const BASE = `${new URL(SITE_URL).pathname.replace(/\/+$/, '')}/`;

export default defineConfig({
  site: SITE_URL,
  base: BASE,
  output: 'static',
  integrations: [
    react(),
    starlight({
      title: 'Maquette Docs',
      description:
        'Documentation for Maquette, a desktop studio that turns concept art into real-time 3D web scenes.',
      favicon: '/icon-64.png',
      // Sets data-os before the first paint, for the header's Download link (src/styles/os.css).
      head: [{ tag: 'script', content: platformScript() }],
      customCss: ['./src/styles/docs.css'],
      components: {
        Header: './src/components/docs/Header.astro',
        MobileMenuFooter: './src/components/docs/MobileMenuFooter.astro',
        PageFrame: './src/components/docs/PageFrame.astro',
        PageTitle: './src/components/docs/PageTitle.astro',
        Sidebar: './src/components/docs/Sidebar.astro',
        ThemeSelect: './src/components/docs/ThemeSelect.astro',
      },
      routeMiddleware: './src/lib/docs-route.ts',
      sidebar: DOCS_GROUPS.map(group => ({
        label: group.label,
        items: group.slugs.map(slug => ({ slug })),
      })),
      tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 3 },
      credits: false,
      expressiveCode: {
        themes: ['vitesse-dark'],
        styleOverrides: {
          borderColor: 'var(--line)',
          borderRadius: '10px',
          codeBackground: 'var(--code)',
          codeFontFamily: 'var(--sl-font-mono)',
          codeFontSize: '0.8125rem',
          codeLineHeight: '1.65',
        },
      },
    }),
    sitemap(),
  ],
  server: { port: PORT },
  build: { inlineStylesheets: 'auto' },
  markdown: { processor: satteri({ hastPlugins: [baseLinks(BASE)] }) },
  vite: {
    plugins: [tailwindcss(), ...basisTranscoder()],
    server: { fs: { allow: ['../..'] } },
    build: { target: 'es2022' },
  },
});
