/**
 * What the site adds to each prop Maquette ships, keyed by asset id: how its
 * file is anchored, which stop turns it or sweeps it, its shadows and the
 * laptop's screen. Where a prop stands and which stops see it come from the
 * shipped layout (see ./manifest.ts).
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
    // The top glared white and the base read as wood: a matte top and an iron base.
    material: {
      roughness: 0.82,
      metalness: 0,
      flatSurface: true,
      lower: { below: 0.62, color: '#26231f', roughness: 0.55, metalness: 0.75 },
    },
  },
  'finished-island-in-painted-stone-and-moss-with-no-wireframe': { ...BOTTOM, spinAt: 'web' },
  'desk-lamp': { ...BOTTOM, receiveShadow: false },
  'book-stack': BOTTOM,
  'brush-cup': BOTTOM,
  'clay-bowl': BOTTOM,
  calipers: {
    ...BOTTOM,
    // Read as brass; calipers are dark steel.
    material: { tint: '#5d6266', tintAmount: 1, metalness: 0.85, roughness: 0.42 },
  },
  'loop-tool': BOTTOM,
  'linen-cloth': {
    ...BOTTOM,
    // Grey from Tripo; the concept's cloth is olive.
    material: { tint: '#6f7446', tintAmount: 1, roughness: 0.95 },
  },
  laptop: {
    ...BOTTOM,
    // Rendered as polished chrome: matte graphite instead.
    material: { color: '#3a3936', metalness: 0.25, roughness: 0.62, flatSurface: true },
    screen: {
      texture: '/scene/laptop-screen.webp',
      size: [0.27, 0.169],
      position: [0, 0.135, -0.12],
      rotation: [-14, 0, 0],
    },
    placeholder: 'The screen is a mock mode capture of the Board; swap in a real project capture.',
  },
};
