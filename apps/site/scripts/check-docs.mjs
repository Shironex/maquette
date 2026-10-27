/**
 * Checks the built site, after `astro build`:
 *
 * - the docs pages load no three.js, which only the landing tour needs,
 * - every root-relative link and asset in every page sits under the site's
 *   base (the path of SITE_URL, which GitHub Pages serves the site from),
 *   lands on a built file, and a #fragment on an id of that page.
 *
 *   node scripts/check-docs.mjs
 */
import { readFileSync, readdirSync, existsSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { SITE_URL } from '../src/lib/site-url.ts';

const dist = resolve(import.meta.dirname, '../dist');
const BASE = new URL(SITE_URL).pathname.replace(/\/+$/, '');
const THREE = /WebGLRenderer/;

const walk = dir =>
  readdirSync(dir).flatMap(name => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });

const pages = walk(dist).filter(file => file.endsWith('.html'));
const docsPages = pages.filter(file => file.startsWith(join(dist, 'docs')));
const problems = [];

/** The chunk files a page loads, including what they import. */
function chunksOf(html) {
  const seen = new Set();
  const queue = [...html.matchAll(/\/_astro\/[\w.-]+\.js/g)].map(m => m[0]);
  while (queue.length > 0) {
    const url = queue.pop();
    if (seen.has(url)) continue;
    seen.add(url);
    const file = join(dist, url);
    if (!existsSync(file)) continue;
    for (const m of readFileSync(file, 'utf8').matchAll(/["'`]\.\/([\w.-]+\.js)["'`]/g)) {
      queue.push(`/_astro/${m[1]}`);
    }
  }
  return [...seen];
}

/** The built file a path under the base lands on. */
const target = path => {
  const clean = decodeURIComponent(path.slice(BASE.length));
  return [join(dist, clean), join(dist, clean, 'index.html')].find(
    file => existsSync(file) && statSync(file).isFile()
  );
};

const ids = new Map();
const idsOf = file => {
  if (!ids.has(file)) {
    const text = readFileSync(file, 'utf8');
    ids.set(file, new Set([...text.matchAll(/\sid="([^"]+)"/g)].map(m => m[1])));
  }
  return ids.get(file);
};

const underBase = path => path === BASE || path.startsWith(`${BASE}/`);

/** Every root-relative URL of a page: links, sources, srcsets and lazy sources. */
function urlsOf(html) {
  const urls = [];
  for (const [, attr, value] of html.matchAll(
    /\s(href|src|data-src|srcset|imagesrcset)="([^"]*)"/g
  )) {
    const values = attr.endsWith('srcset')
      ? value.split(',').map(item => item.trim().split(/\s+/)[0] ?? '')
      : [value];
    urls.push(...values.filter(url => url.startsWith('/') && !url.startsWith('//')));
  }
  return urls;
}

for (const page of docsPages) {
  const name = page.slice(dist.length);
  for (const chunk of chunksOf(readFileSync(page, 'utf8'))) {
    const file = join(dist, chunk);
    if (existsSync(file) && THREE.test(readFileSync(file, 'utf8'))) {
      problems.push(`${name} loads three.js through ${chunk}`);
    }
  }
}

for (const page of pages) {
  const name = page.slice(dist.length);
  for (const href of urlsOf(readFileSync(page, 'utf8'))) {
    const [path = '', fragment] = href.split('#');
    const clean = path.split('?')[0] ?? '';
    if (!underBase(clean)) {
      problems.push(`${name}: ${href} is not under the base ${BASE}/`);
      continue;
    }
    const file = target(clean);
    if (!file) {
      problems.push(`${name}: ${href} points at nothing`);
    } else if (fragment && file.endsWith('.html') && !idsOf(file).has(fragment)) {
      problems.push(`${name}: ${href} has no #${fragment}`);
    }
  }
}

if (problems.length > 0) {
  console.error(problems.join('\n'));
  process.exit(1);
}
console.log(
  `site check: ${pages.length} pages (${docsPages.length} docs pages, no three.js), every link under ${BASE}/ resolves`
);
