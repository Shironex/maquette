/**
 * The layer map of maquette-core: every module belongs to a layer, and a
 * layer may only use itself and the layers below it.
 *
 * scripts/lint-meta/rust-layers.json names the layers (each with the layers
 * it may use directly; the rule follows them down) and puts every module in
 * one. A module path may be a submodule (`inbox::analyse`): the longest
 * matching path wins, for the file that uses and for the path it uses. A
 * root re-export (`crate::CoreError`) counts as the module `lib.rs` takes it
 * from.
 *
 * Every `crate::` path in a core source file is a use: `use crate::x`,
 * `use crate::{x, y::z}` and inline `crate::x::f()`. A use of a layer the
 * file's layer may not reach is an upward use. `pnpm lint:meta` ratchets
 * them per file and module used; a module that is not in the map fails.
 */
export const LAYER_MAP = 'scripts/lint-meta/rust-layers.json';

/** Source text without comments, so doc links do not count as uses. */
function code(source) {
  return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '');
}

/** The path segments after one `crate::`, expanding `{a, b::c}` groups. */
function expand(text, at, prefix, out) {
  const segments = [...prefix];
  let i = at;
  for (;;) {
    const ident = /^[A-Za-z_][A-Za-z0-9_]*/.exec(text.slice(i));
    if (ident) {
      segments.push(ident[0]);
      i += ident[0].length;
      if (text.startsWith('::', i)) {
        i += 2;
        continue;
      }
      out.push(segments);
      return;
    }
    if (text[i] !== '{') {
      out.push(segments);
      return;
    }
    const item = (from, to) => {
      const s = text.slice(from, to);
      if (s.trim() === 'self') out.push(segments);
      else if (s.trim()) expand(text, from + s.search(/\S/), segments, out);
    };
    let depth = 0;
    let start = i + 1;
    for (let j = i; j < text.length; j++) {
      const c = text[j];
      if (c === '{') depth++;
      else if (c === '}' && --depth === 0) return item(start, j);
      else if (c === ',' && depth === 1) {
        item(start, j);
        start = j + 1;
      }
    }
    return;
  }
}

/** Every `crate::` path in a Rust source, as segment lists. */
export function cratePaths(source) {
  const text = code(source);
  const out = [];
  for (const m of text.matchAll(/\bcrate::/g)) expand(text, m.index + m[0].length, [], out);
  return out;
}

/** The longest map key that is a prefix of `segments`, or null. */
function moduleOf(modules, segments) {
  for (let n = segments.length; n > 0; n--) {
    const key = segments.slice(0, n).join('::');
    if (key in modules) return key;
  }
  return null;
}

/** For each layer, every layer it may use: itself and all below it. */
function reachable(layers) {
  const memo = new Map();
  const walk = name => {
    if (memo.has(name)) return memo.get(name);
    if (!(name in layers)) throw new Error(`${LAYER_MAP}: unknown layer "${name}"`);
    const set = new Set([name]);
    memo.set(name, set);
    for (const below of layers[name]) for (const l of walk(below)) set.add(l);
    return set;
  };
  return Object.fromEntries(Object.keys(layers).map(l => [l, walk(l)]));
}

/** Root re-exports from lib.rs: item name to its full path. */
function rootExports(lib) {
  const text = code(lib);
  const exports = {};
  for (const m of text.matchAll(/\bpub use\s+/g)) {
    const paths = [];
    expand(text, m.index + m[0].length, [], paths);
    for (const p of paths) exports[p.at(-1)] = p;
  }
  return exports;
}

/**
 * Upward uses in the crate, as `file -> module` keys with how many times the
 * file uses that module. Throws when a file or a used path has no layer.
 */
export function upwardUses(ctx) {
  const map = JSON.parse(ctx.read(LAYER_MAP) ?? 'null');
  if (!map) throw new Error(`${LAYER_MAP} is missing`);
  const { crate, layers, modules } = map;
  const may = reachable(layers);
  for (const [mod, layer] of Object.entries(modules)) {
    if (!(layer in layers)) throw new Error(`${LAYER_MAP}: ${mod} is in unknown layer "${layer}"`);
  }
  const exports = rootExports(ctx.read(`${crate}/lib.rs`) ?? '');
  const uses = {};
  for (const file of ctx.glob(`${crate}/**/*.rs`).sort()) {
    const self = file
      .slice(crate.length + 1, -'.rs'.length)
      .split('/')
      .filter((s, i, all) => !(s === 'mod' && i === all.length - 1));
    const from = moduleOf(modules, self);
    if (!from) throw new Error(`${file}: module ${self.join('::')} is not in ${LAYER_MAP}`);
    for (const path of cratePaths(ctx.read(file) ?? '')) {
      const to =
        moduleOf(modules, path) ??
        (path[0] in exports ? moduleOf(modules, [...exports[path[0]], ...path.slice(1)]) : null);
      if (!to) throw new Error(`${file}: crate::${path.join('::')} is not in ${LAYER_MAP}`);
      if (may[modules[from]].has(modules[to])) continue;
      const key = `${file} -> ${to}`;
      uses[key] = (uses[key] ?? 0) + 1;
    }
  }
  return uses;
}
