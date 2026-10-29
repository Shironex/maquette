/**
 * The direction of the Rust workspace crates: each crate may depend only on
 * the workspace crates scripts/lint-meta/crate-layers.json allows it, so a
 * lower crate never reaches up into one above it.
 *
 * The dependencies come from `cargo metadata --no-deps`, which reads the
 * manifests without resolving or building anything. Every kind counts
 * (normal, dev and build), and a workspace crate missing from the map fails,
 * so a new crate has to state where it sits.
 */
export const CRATE_MAP = 'scripts/lint-meta/crate-layers.json';

export const METADATA = 'cargo metadata --no-deps --format-version 1 --offline';

/**
 * Violations of the allowed map, given `cargo metadata` output: one per
 * crate missing from the map, map entry naming no crate, and dependency on a
 * workspace crate the map does not allow.
 */
export function crateViolations(metadata, allowed) {
  const members = new Map(metadata.packages.map(p => [p.name, p]));
  const out = [];
  for (const name of [...members.keys()].sort()) {
    if (!(name in allowed)) {
      out.push({ crate: name, message: `${name} is not in ${CRATE_MAP}` });
      continue;
    }
    const deps = new Set(
      members
        .get(name)
        .dependencies.filter(d => members.has(d.name))
        .map(d => d.name)
    );
    for (const dep of [...deps].sort()) {
      if (!allowed[name].includes(dep)) {
        out.push({
          crate: name,
          message: `${name} depends on ${dep}, which ${CRATE_MAP} does not allow. A crate only depends on the crates below it.`,
        });
      }
    }
  }
  for (const name of Object.keys(allowed)) {
    if (!members.has(name)) {
      out.push({
        crate: name,
        message: `${CRATE_MAP} names ${name}, which is not a workspace crate`,
      });
    }
  }
  return out;
}

/** The lint-meta rule: reads the map and the metadata through `ctx`. */
export function crateLayers(ctx) {
  const allowed = JSON.parse(ctx.read(CRATE_MAP) ?? 'null');
  if (!allowed) throw new Error(`${CRATE_MAP} is missing`);
  const r = ctx.exec(METADATA);
  if (r.code !== 0) throw new Error(`${METADATA} failed: ${r.stderr.trim()}`);
  return crateViolations(JSON.parse(r.stdout), allowed);
}
