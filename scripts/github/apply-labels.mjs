/**
 * Creates or updates every label in .github/labels.yml with
 * `gh label create --force`, which edits a label that already exists.
 * Labels not listed there are left alone.
 *
 *   node scripts/github/apply-labels.mjs                  # the repo gh resolves from this folder
 *   node scripts/github/apply-labels.mjs --repo OWNER/NAME
 *   node scripts/github/apply-labels.mjs --dry-run        # print, change nothing
 *
 * Needs the GitHub CLI, signed in with a token that can edit labels. Runs the
 * same on macOS and Windows: gh is spawned with an args array, never a shell.
 */
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const LABELS_FILE = fileURLToPath(new URL('../../.github/labels.yml', import.meta.url));

/**
 * Reads the flat list this file uses: `- name:` starts an entry, and the
 * indented `color:` and `description:` lines belong to it. Values may be
 * wrapped in single or double quotes.
 */
export function parseLabels(text) {
  const labels = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line === '' || line.startsWith('#')) continue;
    const entry = /^(- )?(name|color|description):\s*(.*)$/.exec(line);
    if (!entry) throw new Error(`Cannot read this line of labels.yml: ${raw}`);
    const [, dash, key, value] = entry;
    if (dash) labels.push({});
    const current = labels.at(-1);
    if (!current) throw new Error(`A field comes before the first "- name:" line: ${raw}`);
    current[key] = value.replace(/^(['"])(.*)\1$/, '$2');
  }
  for (const label of labels) {
    if (!label.name || !/^[0-9a-fA-F]{6}$/.test(label.color ?? '')) {
      throw new Error(`A label needs a name and a six digit hex color: ${JSON.stringify(label)}`);
    }
  }
  return labels;
}

function main() {
  const args = process.argv.slice(2);
  const dryRun = args.includes('--dry-run');
  const repoFlag = args.indexOf('--repo');
  const repo = repoFlag === -1 ? null : args[repoFlag + 1];
  if (repoFlag !== -1 && !repo) throw new Error('--repo needs OWNER/NAME');

  const labels = parseLabels(readFileSync(LABELS_FILE, 'utf8'));
  let failed = 0;
  for (const { name, color, description = '' } of labels) {
    const ghArgs = [
      'label',
      'create',
      name,
      '--color',
      color,
      '--description',
      description,
      '--force',
    ];
    if (repo) ghArgs.push('--repo', repo);
    if (dryRun) {
      console.log(`gh ${ghArgs.map(a => JSON.stringify(a)).join(' ')}`);
      continue;
    }
    // On Windows gh is a real gh.exe, so no shell is needed.
    const result = spawnSync('gh', ghArgs, { encoding: 'utf8' });
    if (result.error) throw new Error(`Cannot run gh: ${result.error.message}`);
    if (result.status === 0) {
      console.log(`ok    ${name}`);
    } else {
      failed += 1;
      console.error(`FAIL  ${name}: ${result.stderr.trim()}`);
    }
  }
  console.log(`${labels.length - failed}/${labels.length} labels ${dryRun ? 'listed' : 'applied'}`);
  if (failed > 0) process.exitCode = 1;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) main();
