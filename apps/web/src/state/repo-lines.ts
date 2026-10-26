/**
 * The check lines New project shows under the repo field: what Maquette
 * found in a site repo or a game repo, and where Ship will write in it.
 */
import type { FieldTone } from '@maquette/ui';
import type { GameFormat, GameRepoInspection, RepoInspection, SiteFolders } from '@/ipc/client';

export interface RepoLine {
  tone: FieldTone | 'info';
  text: string;
  /** Muted text after it. */
  detail?: string;
}

/** "a", "a and b", "a, b and c". */
export function listWords(words: readonly string[]): string {
  if (words.length <= 1) return words[0] ?? '';
  return `${words.slice(0, -1).join(', ')} and ${words[words.length - 1]}`;
}

/** Where Ship writes in a site without a maquette.site.json. */
export const LEGACY_SITE: SiteFolders = {
  config: false,
  models: 'public/models',
  posters: 'public/posters',
  data: null,
  error: null,
};

const folder = (path: string) => `${path.replace(/\/+$/, '')}/`;

/** A site repo: git, its maquette.site.json and the folders Ship writes to. */
export function siteRepoLines(repo: RepoInspection | null): RepoLine[] {
  if (!repo) return [];
  if (!repo.exists) return [{ tone: 'missing', text: 'Folder not found.' }];
  const lines: RepoLine[] = [
    repo.isGit
      ? { tone: 'found', text: 'Git repo found.' }
      : {
          tone: 'warning',
          text: 'Not a git repo.',
          detail: 'Ship still writes there, without a commit.',
        },
  ];
  const site = repo.site;
  if (site.error) {
    lines.push({ tone: 'missing', text: 'maquette.site.json cannot be read.', detail: site.error });
  } else if (!site.config) {
    lines.push({
      tone: 'warning',
      text: 'No maquette.site.json at the repo root.',
      detail: 'Add one to choose the folders.',
    });
  }
  const posters = site.posters ? ` and posters to ${folder(site.posters)}` : ', no posters';
  const data = site.data ? `, scene data to ${folder(site.data)}` : ', and no scene data';
  lines.push({
    tone: 'info',
    text: `Ship writes models to ${folder(site.models)}${posters}${data}.`,
    detail: 'The folders come from the repo, never a fixed path.',
  });
  if (repo.looksLikeGame) {
    lines.push({
      tone: 'warning',
      text: 'This repo has a CMakeLists.txt.',
      detail: 'If it is a game, pick Game under Ships to.',
    });
  }
  return lines;
}

const FORMAT_NAME: Record<GameFormat, string> = { glb: '.glb', obj: '.obj' };

/** What Game export writes for this format. */
export function exportSentence(format: GameFormat): string {
  return format === 'obj'
    ? 'Game export writes .obj with its .mtl and PNG or JPG textures.'
    : 'Game export writes plain .glb with PNG and JPG textures.';
}

/** A game repo: git, CMake, the model folder and its format, the loader. */
export function gameRepoLines(
  repo: GameRepoInspection | null,
  format: GameFormat,
  modelsDir: string
): RepoLine[] {
  if (!repo) return [];
  if (!repo.exists) return [{ tone: 'missing', text: 'Folder not found.' }];
  const lines: RepoLine[] = [
    repo.isGit
      ? {
          tone: 'found',
          text: 'Git repo found.',
          detail: repo.branch ? `On ${repo.branch}.` : undefined,
        }
      : {
          tone: 'warning',
          text: 'Not a git repo.',
          detail: 'Ship still writes there, without a commit.',
        },
  ];
  if (repo.cmake) {
    lines.push({
      tone: 'found',
      text: 'CMake project found.',
      detail: repo.targets.length
        ? `CMakeLists.txt builds ${listWords(repo.targets)}.`
        : 'CMakeLists.txt names no executable.',
    });
  } else {
    lines.push({ tone: 'warning', text: 'No CMakeLists.txt at the root.' });
  }
  const count = repo.glbFiles + repo.objFiles;
  if (repo.modelsDir && repo.format) {
    const files = repo.format === 'glb' ? repo.glbFiles : repo.objFiles;
    const where = repo.folders.length
      ? ` in ${listWords(repo.folders.map(f => (f ? folder(f) : 'the folder itself')))}`
      : '';
    const loader = repo.loader ? `, read with ${repo.loader}` : '';
    lines.push({
      tone: 'found',
      text: `Models are ${FORMAT_NAME[repo.format]} in ${folder(repo.modelsDir)}`,
      detail: `(${files} ${files === 1 ? 'file' : 'files'}${where})${loader}.`,
    });
  } else {
    lines.push({
      tone: 'warning',
      text: count ? 'Models found, but no model folder.' : 'No models found.',
      detail: `Ship writes to ${folder(modelsDir)}.`,
    });
  }
  const missing = [repo.readsKtx2 ? null : 'KTX2', repo.readsMeshopt ? null : 'meshopt'].filter(
    (w): w is string => Boolean(w)
  );
  lines.push({
    tone: 'info',
    text: missing.length ? `The engine cannot read ${listWords(missing)}.` : 'Textures stay plain.',
    detail: exportSentence(format),
  });
  lines.push({
    tone: 'info',
    text: `Ship writes to ${folder(modelsDir)} and only replaces files it wrote.`,
    detail: 'The other files stay as they are.',
  });
  return lines;
}
