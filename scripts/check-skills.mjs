/**
 * Keeps the agent skills honest: every Maquette tool a skill names must exist.
 *
 * The authoritative tool list is the snapshot `cargo test -p maquette-desktop`
 * writes from the server's tool router (apps/desktop/src-tauri/tool-names.txt),
 * which the MCP handshake test also checks; not the docs page. For every
 * plugins/<plugin>/skills/<skill>/SKILL.md (and the Markdown beside it) the check:
 *
 * - takes each inline `backticked` snake_case word whose first segment is a verb that
 *   starts a real tool (get, set, list, ...) and fails when it is not in the tool list;
 * - needs `name` (equal to the folder) and `description` in the frontmatter, and a
 *   description that says when to use the skill;
 * - keeps each SKILL.md at 250 lines or fewer;
 * - needs every SKILL.md to carry the plugin's shared/before-you-start.md word for word, when it has one;
 * - rejects em and en dashes in skills and in the plugin files;
 * - needs the marketplace entry to point at a plugin folder whose plugin.json has the same name;
 * - needs the plugin's MCP server entry to read its token from the environment, never a literal.
 *
 *   node scripts/check-skills.mjs      exit 1 on any violation
 *   pnpm lint:meta                     runs it too
 */
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const TOOL_NAMES = 'apps/desktop/src-tauri/tool-names.txt';
const MAX_LINES = 250;
const DASHES = /[\u2013\u2014]/;
const SHARED_BLOCK = 'shared/before-you-start.md';
const SNAKE = /^[a-z][a-z0-9]*(?:_[a-z0-9]+)+$/;

const read = (root, rel) => readFileSync(join(root, rel), 'utf8').replace(/\r\n/g, '\n');

/** The tool names the server lists, read from the generated tool-names snapshot. */
export function serverTools(root) {
  const names = read(root, TOOL_NAMES)
    .split('\n')
    .map(line => line.trim())
    .filter(line => line && !line.startsWith('#'));
  if (names.length < 10) throw new Error(`${TOOL_NAMES}: only ${names.length} tool names found`);
  return new Set(names);
}

/** The pages that document every tool by name: the README and the docs site's MCP tools page. */
export const TOOL_DOCS = ['README.md', 'apps/site/src/content/docs/docs/agents/mcp-tools.md'];

/** One violation per tool a documenting page never names in backticks. */
export function undocumentedTools(root, pages = TOOL_DOCS) {
  const tools = serverTools(root);
  const violations = [];
  for (const page of pages) {
    const named = new Set(backticked(read(root, page)));
    for (const tool of tools)
      if (!named.has(tool))
        violations.push({
          rule: 'tools-documented',
          file: page,
          message: `does not name \`${tool}\`, a tool of the MCP server (${TOOL_NAMES})`,
        });
  }
  return violations;
}

/** One violation per page that states a tool count other than the server's real one. */
export function staleToolCount(root, pages = TOOL_DOCS) {
  const real = serverTools(root).size;
  const violations = [];
  for (const page of pages) {
    for (const m of read(root, page).matchAll(
      /\b(?:offers|lists|has|with) (\d+) (?:MCP )?tools\b/g
    ))
      if (Number(m[1]) !== real)
        violations.push({
          rule: 'tools-documented',
          file: page,
          message: `says ${m[1]} tools, the MCP server lists ${real} (${TOOL_NAMES})`,
        });
  }
  return violations;
}

/** Inline backticked tokens of a Markdown text, fenced blocks included. */
export function backticked(text) {
  return [...text.matchAll(/`([^`\n]+)`/g)].map(m => m[1].trim());
}

/** Backticked words shaped like a tool name, judged against the tool list. */
export function toolMentions(text, tools) {
  const verbs = new Set([...tools].map(t => t.split('_')[0]));
  return [...new Set(backticked(text))].filter(t => SNAKE.test(t) && verbs.has(t.split('_')[0]));
}

function walk(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

function frontmatter(text) {
  const m = /^---\n([\s\S]*?)\n---\n/.exec(text);
  if (!m) return null;
  const fields = {};
  for (const line of m[1].split('\n')) {
    const kv = /^([a-z-]+):\s*(.*)$/.exec(line);
    if (kv) fields[kv[1]] = kv[2].trim();
  }
  return fields;
}

/** All violations under `root`, as { rule, file, message }. */
export function checkSkills(root) {
  const violations = [];
  const add = (file, message) => violations.push({ rule: 'skills-honest', file, message });
  const pluginsDir = join(root, 'plugins');
  if (!existsSync(pluginsDir)) return violations;
  const tools = serverTools(root);

  for (const plugin of readdirSync(pluginsDir)) {
    const base = join(pluginsDir, plugin);
    if (!statSync(base).isDirectory()) continue;
    const rel = p => relative(root, p).replaceAll('\\', '/');

    for (const file of walk(base)) {
      if (!/\.(md|json)$/.test(file)) continue;
      const text = readFileSync(file, 'utf8');
      if (DASHES.test(text))
        add(rel(file), 'contains an em or en dash; use a comma, colon or period');
      if (file.endsWith('.md'))
        for (const name of toolMentions(text, tools))
          if (!tools.has(name))
            add(rel(file), `names \`${name}\`, which is not a tool of the MCP server`);
    }

    const skillsDir = join(base, 'skills');
    const skills = existsSync(skillsDir) ? readdirSync(skillsDir) : [];
    if (skills.length === 0) add(rel(base), 'the plugin has no skills');
    const sharedPath = join(base, SHARED_BLOCK);
    const shared = existsSync(sharedPath) ? read(root, rel(sharedPath)).trim() : null;
    for (const skill of skills) {
      const file = join(skillsDir, skill, 'SKILL.md');
      if (!existsSync(file)) {
        add(rel(join(skillsDir, skill)), 'has no SKILL.md');
        continue;
      }
      const text = read(root, rel(file));
      const fm = frontmatter(text);
      if (!fm) add(rel(file), 'needs frontmatter with name and description');
      else {
        if (fm.name !== skill)
          add(rel(file), `name must equal the folder (${skill}), found "${fm.name ?? ''}"`);
        if (!fm.description || !/\buse\b/i.test(fm.description))
          add(rel(file), 'description must say when to use the skill ("Use when ...")');
      }
      if (shared && !text.includes(shared))
        add(rel(file), `must carry ${SHARED_BLOCK} word for word`);
      const lines = text.split('\n').length;
      if (lines > MAX_LINES + 1) add(rel(file), `${lines} lines exceeds the ${MAX_LINES} line cap`);
    }

    const manifestPath = join(base, '.claude-plugin/plugin.json');
    const manifest = existsSync(manifestPath)
      ? JSON.parse(readFileSync(manifestPath, 'utf8'))
      : null;
    if (!manifest) add(rel(base), 'has no .claude-plugin/plugin.json');
    else if (manifest.name !== plugin)
      add(rel(manifestPath), `name must equal the folder (${plugin})`);

    const mcpPath = join(base, '.mcp.json');
    if (existsSync(mcpPath)) {
      const server = JSON.parse(readFileSync(mcpPath, 'utf8')).mcpServers?.maquette;
      const auth = server?.headers?.Authorization ?? '';
      if (!/^Bearer \$\{[A-Z_]+\}$/.test(auth))
        add(rel(mcpPath), 'the token must come from an environment variable');
      if (!/\$\{[A-Z_]+(:-\d+)?\}/.test(server?.url ?? ''))
        add(rel(mcpPath), 'the port must come from an environment variable');
    }
  }

  const marketPath = join(root, '.claude-plugin/marketplace.json');
  if (existsSync(marketPath)) {
    const market = JSON.parse(readFileSync(marketPath, 'utf8'));
    for (const entry of market.plugins ?? []) {
      const dir = resolve(root, entry.source ?? '');
      const manifest = join(dir, '.claude-plugin/plugin.json');
      if (!existsSync(manifest))
        add(
          '.claude-plugin/marketplace.json',
          `${entry.name}: source ${entry.source} has no plugin`
        );
      else if (JSON.parse(readFileSync(manifest, 'utf8')).name !== entry.name)
        add(
          '.claude-plugin/marketplace.json',
          `${entry.name}: differs from the name in ${entry.source}'s plugin.json`
        );
    }
  }
  return violations;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const root = resolve(import.meta.dirname, '..');
  const violations = checkSkills(root);
  for (const v of violations) console.error(`${v.rule}  ${v.file}  ${v.message}`);
  if (violations.length > 0) {
    console.error(`\ncheck-skills: ${violations.length} violation(s)`);
    process.exit(1);
  }
  console.log(`check-skills: ${serverTools(root).size} server tools, skills consistent`);
}
