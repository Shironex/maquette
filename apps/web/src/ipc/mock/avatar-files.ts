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
  const springs = `${dir}/springs`;
  return {
    [`${springs}/find/candidates.webp`]: 'springs_candidates.webp',
    [`${springs}/chain/sheet_before.webp`]: 'springs_before.webp',
    [`${springs}/chain/sheet_after.webp`]: 'springs_after.webp',
    [`${dir}/sheet_before.webp`]: 'sheet_before.webp',
    [`${dir}/sheet_after.webp`]: 'sheet_after.webp',
    [`${dir}/reweight/sheet_before.webp`]: 'reweight_before.webp',
    [`${dir}/reweight/sheet_after.webp`]: 'reweight_after.webp',
  };
}

/**
 * The `.vrm` every mock build "writes", by its path under each project:
 * the box humanoid test fixture written as VRM 1.0, with spring bones in
 * the springs scenarios.
 */
export function avatarVrmFiles(
  root: string,
  projects: readonly string[],
  springs = false
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const project of projects) {
    for (const asset of ['elsheria', 'clay-figure']) {
      out[`${root}/projects/${project}/exports/${asset}.vrm`] = 'box_humanoid.vrm';
    }
    // With springs, the box humanoid with a strip down its back and a chain in it.
    if (springs) {
      out[`${root}/projects/${project}/exports/elsheria.vrm`] = 'box_humanoid_springs.vrm';
    }
  }
  return out;
}
