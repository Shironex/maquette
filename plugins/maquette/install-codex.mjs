/**
 * Makes the plugin's skills visible to Codex, from the same source Claude Code reads.
 *
 * Codex discovers skills in `$HOME/.agents/skills` (every repository) and in
 * `<repo>/.agents/skills` (one repository), and follows symlinked skill folders. So the
 * skills stay in plugins/maquette/skills and this script links each one there. Nothing is
 * duplicated, and a `git pull` updates them.
 *
 *   node plugins/maquette/install-codex.mjs                 link into ~/.agents/skills
 *   node plugins/maquette/install-codex.mjs --repo <path>   link into <path>/.agents/skills
 *   node plugins/maquette/install-codex.mjs --copy          copy instead of linking
 *   node plugins/maquette/install-codex.mjs --remove        remove what this script installed
 *
 * On Windows a link is a directory junction, which needs no administrator rights. Restart
 * Codex if a skill does not show up in `/skills` (type `$` to mention one).
 */
import {
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readlinkSync,
  rmSync,
  symlinkSync,
} from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

const skillsDir = resolve(import.meta.dirname, 'skills');
const args = process.argv.slice(2);
const flag = name => args.includes(name);
const repoAt = args.indexOf('--repo');
const root = repoAt >= 0 ? resolve(args[repoAt + 1] ?? '') : homedir();
const dest = join(root, '.agents', 'skills');

const names = readdirSync(skillsDir).filter(n => existsSync(join(skillsDir, n, 'SKILL.md')));
/** True when `target` is a link to this plugin's copy of the skill `name`. */
const isOurLink = (target, name) => {
  try {
    return (
      lstatSync(target).isSymbolicLink() &&
      resolve(dest, readlinkSync(target)) === join(skillsDir, name)
    );
  } catch {
    return false;
  }
};

if (flag('--remove')) {
  for (const name of names) {
    const target = join(dest, name);
    if (isOurLink(target, name) || (existsSync(target) && flag('--copy'))) {
      rmSync(target, { recursive: true, force: true });
      console.log(`removed ${target}`);
    }
  }
  process.exit(0);
}

mkdirSync(dest, { recursive: true });
for (const name of names) {
  const source = join(skillsDir, name);
  const target = join(dest, name);
  if (lstatSync(target, { throwIfNoEntry: false })) {
    if (isOurLink(target, name)) {
      console.log(`already linked: ${target}`);
      continue;
    }
    // A copy made by an earlier --copy run is refreshed; anything else is left alone.
    if (
      flag('--copy') &&
      !lstatSync(target).isSymbolicLink() &&
      existsSync(join(target, 'SKILL.md'))
    ) {
      rmSync(target, { recursive: true, force: true });
    } else {
      console.error(
        `skipped ${target}: it exists and is not a link to this plugin. Remove it first.`
      );
      process.exitCode = 1;
      continue;
    }
  }
  if (flag('--copy')) cpSync(source, target, { recursive: true });
  else symlinkSync(source, target, process.platform === 'win32' ? 'junction' : 'dir');
  console.log(`${flag('--copy') ? 'copied' : 'linked'} ${target}`);
}
console.log(
  `\n${names.length} skills in ${dest}. Add the MCP server to ~/.codex/config.toml (see the Connect an agent page).`
);
