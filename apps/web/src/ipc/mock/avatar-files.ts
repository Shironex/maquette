/**
 * The files the mock Avatar view serves: the bundled sheets under the
 * avatar cache and the `.vrm` every mock build "writes".
 */

/** Where the mock's normalise "writes" its sheets, under the cache. */
export function avatarCacheDir(root: string, project: string, asset: string) {
  return `${root}/cache/avatar/${project}/${asset}`;
}

/** The bundled sheet each cached sheet path serves. */
export function avatarSheetFiles(root: string): Record<string, string> {
  const dir = avatarCacheDir(root, 'frostveil', 'elsheria');
  return {
    [`${dir}/sheet_before.webp`]: 'sheet_before.webp',
    [`${dir}/sheet_after.webp`]: 'sheet_after.webp',
    [`${dir}/reweight/sheet_before.webp`]: 'reweight_before.webp',
    [`${dir}/reweight/sheet_after.webp`]: 'reweight_after.webp',
  };
}

/**
 * The `.vrm` every mock build "writes", by its path under each project:
 * the box humanoid test fixture written as VRM 1.0.
 */
export function avatarVrmFiles(root: string, projects: readonly string[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const project of projects) {
    for (const asset of ['elsheria', 'clay-figure']) {
      out[`${root}/projects/${project}/exports/${asset}.vrm`] = 'box_humanoid.vrm';
    }
  }
  return out;
}
