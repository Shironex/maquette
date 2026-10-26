/**
 * Checks the built site, after `astro build`:
 *
 * - the docs pages load no three.js, which only the landing tour needs,
 * - every root-relative link in the docs pages lands on a built file, and a
 *   #fragment on an id of that page.
 *
 *   node scripts/check-docs.mjs
 */
import { readFileSync, readdirSync, existsSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const dist = resolve(import.meta.dirname, '../dist');
const THREE = /WebGLRenderer/;

const walk = dir =>
  readdirSync(dir).flatMap(name => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });

const pages = walk(join(dist, 'docs')).filter(file => file.endsWith('.html'));
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

const target = path => {
  const clean = decodeURIComponent(path);
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

for (const page of pages) {
  const name = page.slice(dist.length);
  const html = readFileSync(page, 'utf8');
  for (const chunk of chunksOf(html)) {
    const file = join(dist, chunk);
    if (existsSync(file) && THREE.test(readFileSync(file, 'utf8'))) {
      problems.push(`${name} loads three.js through ${chunk}`);
    }
  }
  for (const [, href] of html.matchAll(/\shref="(\/[^"]*)"/g)) {
    const [path = '', fragment] = href.split('#');
    const file = path === '' ? page : target(path.split('?')[0] ?? '');
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
console.log(`docs check: ${pages.length} pages, no three.js, every link resolves`);
