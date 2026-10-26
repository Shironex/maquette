import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { basisTranscoder } from './vite/basis-transcoder.ts';
import { DOCS_GROUPS } from './src/lib/docs-nav.ts';
import { SITE_URL } from './src/lib/site-url.ts';

// Next to the web app's 15190 and its worktree ports; preview uses the same.
const PORT = 15292;

// The site is served from the root of its dev server, so there is no `base`,
// and the sitemap would lose the path of SITE_URL without this.
const withSitePath = (url: string) => url.replace(new URL(SITE_URL).origin, SITE_URL);

export default defineConfig({
  site: SITE_URL,
  output: 'static',
  integrations: [
    react(),
    starlight({
      title: 'Maquette Docs',
      description:
        'Documentation for Maquette, a desktop studio that turns concept art into real-time 3D web scenes.',
      favicon: '/icon-64.png',
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
    sitemap({ serialize: item => ({ ...item, url: withSitePath(item.url) }) }),
  ],
  server: { port: PORT },
  build: { inlineStylesheets: 'auto' },
  vite: {
    plugins: [tailwindcss(), ...basisTranscoder()],
    server: { fs: { allow: ['../..'] } },
    build: { target: 'es2022' },
  },
});
