/**
 * What the site adds to each prop Maquette ships, keyed by asset id: how its
 * file is anchored, which stop turns it or sweeps it, its shadows and the
 * laptop's screen. Where a prop stands and which stops see it come from the
 * shipped layout (see ./manifest.ts). Colours, roughness, metal and glow are
 * the project's material tweaks, which the pipeline bakes into the files.
 */
import type { SiteExtras } from './layout';

export type PropExtras = Partial<Omit<SiteExtras, 'file' | 'exportedInPlace'>>;

/**
 * The props on the bench today came from Tripo with an offset left over
 * from an earlier scene, so they stand on the bottom centre of their bounds.
 */
const BOTTOM: PropExtras = { anchor: 'bottom-center' };

export const PROP_EXTRAS: Readonly<Record<string, PropExtras>> = {
  'plaster-plinth': BOTTOM,
  'clay-island-maquette': { ...BOTTOM, sweepAt: 'blender' },
  turntable: {
    ...BOTTOM,
    spinAt: 'web',
    // The base read as wood: an iron band up to the top. The matte top is
    // the project's material tweak, baked into the shipped file.
    material: { lower: { below: 0.62, color: '#26231f', roughness: 0.55, metalness: 0.75 } },
  },
  'finished-island-in-painted-stone-and-moss-with-no-wireframe': { ...BOTTOM, spinAt: 'web' },
  'desk-lamp': { ...BOTTOM, receiveShadow: false },
  'book-stack': BOTTOM,
  'brush-cup': BOTTOM,
  'clay-bowl': BOTTOM,
  calipers: BOTTOM,
  'loop-tool': BOTTOM,
  'linen-cloth': BOTTOM,
  laptop: {
    ...BOTTOM,
    screen: {
      texture: '/scene/laptop-screen.webp',
      size: [0.27, 0.169],
      position: [0, 0.135, -0.12],
      rotation: [-14, 0, 0],
    },
    placeholder: 'The screen is a mock mode capture of the Board; swap in a real project capture.',
  },
};
