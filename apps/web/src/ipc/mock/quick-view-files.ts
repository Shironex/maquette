/**
 * The model files the 3D quick view loads in mock mode: every version of
 * the mock projects' assets and of the library's models, mapped by stem to
 * one of the bundled models under `assets/mock/models`.
 */

/** A model stem (`crystal_lantern`) to the bundled model that stands in for it. */
const STAND_IN: Record<string, string> = {
  elsheria: 'elsheria.glb',
  crystal_lantern: 'frost_lantern.glb',
  observatory_arch: 'sigil_gate.glb',
  veil_platform: 'veil_gate.glb',
  veil_gate: 'veil_gate.glb',
  guardian_statue: 'clay_figure.glb',
  rune_obelisk: 'sigil_gate_frame.glb',
  ice_crystals: 'ice_crystals.glb',
};

/** More versions than any mock asset or library model has. */
const VERSIONS = 8;

/** Each version path, absolute, to the bundled model file it shows. */
export function quickViewFiles(root: string, projects: readonly string[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [stem, model] of Object.entries(STAND_IN)) {
    for (let n = 1; n <= VERSIONS; n++) {
      for (const project of projects)
        out[`${root}/projects/${project}/models/${stem}_v${n}.glb`] = model;
      out[`${root}/library/${stem.replace(/_/g, '-')}/models/${stem}_v${n}.glb`] = model;
    }
  }
  return out;
}
