/**
 * The showcase: real files from a game project made in Maquette, from the
 * concept sheet to an animated character. The images are the project's own
 * concepts, refs and thumbnails, scaled down; the character renders come from
 * its shipped models. Kept apart from copy.ts so the page's words stay short.
 */
import { withBase } from '../lib/with-base';

export interface Tile {
  src: string;
  width: number;
  height: number;
  alt: string;
  /** A smaller copy for narrow screens, with its width. */
  small?: { src: string; width: number };
  /** A short loop that plays over the still once it is on screen. */
  video?: string;
  /** A note in the tile's corner, as the app's viewers show one. */
  badge?: string;
}

export interface Step {
  number: string;
  title: string;
  tool: string;
  caption: string;
  /** How the tiles sit: one image, a 2 x 2 grid, or a row of one height. */
  layout: 'single' | 'grid' | 'row';
  tiles: readonly Tile[];
}

const file = (name: string) => withBase(`/scene/showcase/${name}`);

const square = (name: string, size: number, alt: string): Tile => ({
  src: file(`${name}.webp`),
  width: size,
  height: size,
  alt,
});

/** A modular kit: one concept sheet, an image per piece, then a model per piece. */
export const KIT: readonly Step[] = [
  {
    number: '01',
    title: 'Concept sheet',
    tool: 'Lab',
    caption: 'ten modular pieces on one sheet',
    layout: 'single',
    tiles: [
      {
        src: file('kit-sheet-1120.webp'),
        width: 1120,
        height: 631,
        small: { src: file('kit-sheet-640.webp'), width: 640 },
        alt: 'A concept sheet of ten modular pieces for a frozen ritual plaza: rune floor rings, standing stones, a shard pillar, steps, a bridge and an ice crystal wall, on light grey.',
      },
    ],
  },
  {
    number: '02',
    title: 'Object refs',
    tool: 'Make object refs',
    caption: 'one clean image per piece',
    layout: 'grid',
    tiles: [
      square('kit-ref-1', 320, 'A curved floor ring of carved stone with a violet rune line.'),
      square('kit-ref-2', 320, 'A tall rough standing stone with a glowing rune.'),
      square('kit-ref-3', 320, 'A broken rune pillar with a floating ice shard.'),
      square('kit-ref-4', 320, 'A wall of blue ice crystals on snowy rocks.'),
    ],
  },
  {
    number: '03',
    title: 'Models',
    tool: 'By hand',
    caption: 'made on Tripo, thumbnails from Maquette',
    layout: 'grid',
    tiles: [
      square('kit-model-1', 256, 'The curved floor ring as a 3D model.'),
      square('kit-model-2', 256, 'The standing stone as a 3D model.'),
      square('kit-model-3', 256, 'The shard pillar as a 3D model.'),
      square('kit-model-4', 256, 'The ice crystal wall as a 3D model.'),
    ],
  },
];

/** Characters: one from refs by view to a model, and a rigged one playing a clip. */
export const CHARACTER: readonly Step[] = [
  {
    number: '04',
    title: 'Refs by view',
    tool: 'Refs',
    caption: 'front, left and back from one turnaround',
    layout: 'row',
    tiles: [
      {
        src: file('knight-front.webp'),
        width: 343,
        height: 640,
        alt: 'Front view of a knight in frosted plate armour with a violet cloak.',
      },
      {
        src: file('knight-left.webp'),
        width: 258,
        height: 640,
        alt: 'The same knight from the left.',
      },
      {
        src: file('knight-back.webp'),
        width: 362,
        height: 640,
        alt: 'The same knight from the back, the cloak with a silver sigil.',
      },
    ],
  },
  {
    number: '05',
    title: 'Model',
    tool: 'By hand',
    caption: 'Tripo multi-view from those three',
    layout: 'row',
    tiles: [
      {
        src: file('knight-model.webp'),
        width: 400,
        height: 640,
        alt: 'The knight as a textured 3D model, seen from three quarters.',
      },
    ],
  },
  {
    number: '06',
    title: 'Animated',
    tool: 'Rigged',
    caption: 'one of the 187 clips in her model',
    layout: 'row',
    tiles: [
      {
        src: file('character-clip.webp'),
        width: 400,
        height: 640,
        video: file('character-clip.mp4'),
        badge: '2.7 s loop',
        alt: 'A sorceress in white and violet robes casting a two handed spell, one of the clips in her rigged model.',
      },
    ],
  },
];
