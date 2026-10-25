/**
 * Mock inbox: four Tripo downloads with the analysis and the check rows the
 * Rust side produces for them. The character carries the real numbers of a
 * Tripo auto rigged girl: 94.6k triangles, 65 Mixamo bones and twin tails
 * weighted to the index fingers.
 */
import type { Check, Fixup, GlbReport, InboxItem, TextureInfo } from '../client';

const MIN = 60_000;

function mixamoJoints(): string[] {
  const names = ['Hips', 'Spine', 'Spine1', 'Spine2', 'Neck', 'Head', 'HeadTop_End'];
  for (const side of ['Left', 'Right']) {
    names.push(`${side}Shoulder`, `${side}Arm`, `${side}ForeArm`, `${side}Hand`);
    for (const finger of ['Thumb', 'Index', 'Middle', 'Ring', 'Pinky']) {
      for (let n = 1; n <= 4; n += 1) names.push(`${side}Hand${finger}${n}`);
    }
  }
  for (const side of ['Left', 'Right']) {
    names.push(`${side}UpLeg`, `${side}Leg`, `${side}Foot`, `${side}ToeBase`, `${side}Toe_End`);
  }
  return names.map(n => `mixamorig:${n}`);
}

function pbrTextures(size: number, bytes: number): TextureInfo[] {
  return [
    {
      name: 'base_color',
      mime: 'image/jpeg',
      width: size,
      height: size,
      bytes,
      uses: ['base color'],
    },
    {
      name: 'normal',
      mime: 'image/png',
      width: size,
      height: size,
      bytes: bytes * 1.4,
      uses: ['normal'],
    },
    {
      name: 'orm',
      mime: 'image/png',
      width: size,
      height: size,
      bytes: bytes * 0.8,
      uses: ['metal and roughness', 'occlusion'],
    },
  ];
}

/** The bounds of a model `size` big standing centred on the origin, unless `partial` says. */
function report(partial: Partial<GlbReport>): GlbReport {
  const [w, h, d] = (partial.size ?? [1, 1, 1]).map(v => v ?? 0) as [number, number, number];
  return {
    min: [-w / 2, 0, -d / 2],
    max: [w / 2, h, d / 2],
    objects: 1,
    transformed: 0,
    bytes: 0,
    generator: 'Tripo',
    triangles: 0,
    vertices: 0,
    meshes: 1,
    primitives: 1,
    materials: 1,
    textures: [],
    maxTexture: 0,
    skins: 0,
    joints: 0,
    jointNames: [],
    rig: 'none',
    animations: [],
    size: [1, 1, 1],
    pbr: {
      baseColor: true,
      metallicRoughness: true,
      normal: true,
      occlusion: true,
      emissive: false,
    },
    extensions: [],
    geometryReadable: true,
    islands: { mesh: 'mesh', islands: 1, groups: 1 },
    fingerWeights: null,
    ...partial,
  };
}

const c = (
  id: string,
  status: Check['status'],
  title: string,
  detail: string | null = null,
  fixes: Fixup[] = []
): Check => ({
  id,
  status,
  title,
  detail,
  fixes,
});

/** Where the thumbnail service caches renders, next to the inbox folder. */
export function thumbsDir(inbox: string): string {
  return `${inbox.replace(/\/inbox$/, '')}/cache/thumbs`;
}

export function inboxItems(dir: string, now: number): InboxItem[] {
  const joints = mixamoJoints();
  const thumbs = thumbsDir(dir);
  const character: InboxItem = {
    id: 'character.glb',
    name: 'Character',
    file: `${dir}/character.glb`,
    bytes: 18.4e6,
    modifiedMs: now - 4 * MIN,
    kind: 'character',
    kindGuessed: true,
    kindFromSend: false,
    state: 'ready',
    report: report({
      bytes: 18.4e6,
      triangles: 94_612,
      vertices: 51_870,
      textures: pbrTextures(2048, 2.1e6),
      maxTexture: 2048,
      skins: 1,
      joints: 65,
      jointNames: joints,
      rig: 'mixamo',
      size: [0.62, 1.58, 0.31],
      islands: { mesh: 'tripo_node', islands: 3, groups: 1 },
      fingerWeights: {
        vertices: 1284,
        joints: [
          { name: 'mixamorig:LeftHandIndex1', vertices: 652 },
          { name: 'mixamorig:RightHandIndex1', vertices: 632 },
        ],
        reach: 0.237,
      },
    }),
    checks: [
      c(
        'triangles',
        'warning',
        '94.6k triangles',
        'Over the 60k per model budget. Decimate it here, or raise the budget in Settings.',
        [{ op: 'decimate', triangles: 60_000 }]
      ),
      c('textures', 'pass', '3 textures, largest 2048 px', 'At or under the 2048 px default.'),
      c('rig', 'pass', 'Mixamo rig found', '65 joints, ready for Mixamo animations.'),
      c(
        'pbr',
        'pass',
        'PBR maps present',
        'Base color, metal and roughness, normal, plus occlusion.'
      ),
      c('merged', 'pass', 'One object per mesh'),
      c('origin', 'pass', 'Origin at the bottom centre'),
      c(
        'finger-weights',
        'fail',
        'Hair or tail vertices weighted to finger bones',
        '1284 vertices far from the hands follow LeftHandIndex1 (652), RightHandIndex1 (632). Reweight them to the head in Blender.'
      ),
      c(
        'animations',
        'warning',
        'No animation clips yet',
        'The rig is ready. Add clips in Blender after Accept.'
      ),
    ],
    worst: 'fail',
    error: null,
    thumbnail: `${thumbs}/character.webp`,
    thumbnailState: 'ready',
    link: null,
  };
  const sakura: InboxItem = {
    id: 'sakura_large.glb',
    name: 'Sakura large',
    file: `${dir}/sakura_large.glb`,
    bytes: 9.8e6,
    modifiedMs: now - 9 * MIN,
    kind: 'prop',
    kindGuessed: true,
    kindFromSend: false,
    state: 'ready',
    report: report({
      bytes: 9.8e6,
      triangles: 52_104,
      vertices: 28_840,
      textures: pbrTextures(2048, 1.8e6),
      maxTexture: 2048,
      size: [11.68, 7.97, 7.63],
      meshes: 3,
      objects: 3,
      transformed: 2,
    }),
    checks: [
      c('triangles', 'pass', '52.1k triangles', 'Within the 60k per model budget.'),
      c('textures', 'pass', '3 textures, largest 2048 px', 'At or under the 2048 px default.'),
      c('rig', 'pass', 'No rig, as expected'),
      c(
        'pbr',
        'pass',
        'PBR maps present',
        'Base color, metal and roughness, normal, plus occlusion.'
      ),
      c('merged', 'pass', 'One object per mesh'),
      c('origin', 'pass', 'Origin at the bottom centre'),
      c(
        'objects',
        'pass',
        '3 objects',
        'Merging them into one mesh saves draw calls when they never move apart.',
        [{ op: 'merge', byMaterial: false }, { op: 'apply-transforms' }]
      ),
    ],
    worst: 'pass',
    error: null,
    thumbnail: `${thumbs}/sakura-large.webp`,
    thumbnailState: 'ready',
    link: null,
  };
  const shrubs: InboxItem = {
    id: 'flower_shrubs.glb',
    name: 'Flower shrubs',
    file: `${dir}/flower_shrubs.glb`,
    bytes: 6.1e6,
    modifiedMs: now - 15 * MIN,
    kind: 'prop',
    kindGuessed: false,
    kindFromSend: true,
    state: 'ready',
    report: report({
      bytes: 6.1e6,
      triangles: 28_416,
      vertices: 15_302,
      textures: pbrTextures(4096, 4.2e6).slice(0, 1),
      maxTexture: 4096,
      pbr: {
        baseColor: true,
        metallicRoughness: false,
        normal: false,
        occlusion: false,
        emissive: false,
      },
      islands: { mesh: 'shrubs', islands: 41, groups: 3 },
      size: [1.58, 0.67, 1.32],
      min: [-0.2, 0.05, -0.66],
      max: [1.38, 0.72, 0.66],
    }),
    checks: [
      c('triangles', 'pass', '28.4k triangles', 'Within the 60k per model budget.'),
      c(
        'textures',
        'warning',
        '1 texture, largest 4096 px',
        '1 texture is over the 2048 px default: base_color. It was sent with detailed textures. Compress resizes them, or set a per model size.',
        [{ op: 'shrink-textures', maxSize: 2048 }]
      ),
      c('rig', 'pass', 'No rig, as expected'),
      c(
        'pbr',
        'pass',
        'PBR off, as sent',
        'Sent with PBR off, so only a base color map is expected.'
      ),
      c(
        'merged',
        'warning',
        'Looks like 3 objects in one mesh',
        'shrubs has 3 far apart parts. Split them in Blender (Separate by loose parts) so each can move, or remove the small ones when they are stray bits.',
        [{ op: 'remove-loose', minShare: 0.02 }]
      ),
      c(
        'origin',
        'warning',
        'Origin is off the bottom centre',
        'Its bottom centre sits 59 cm from the origin, so it floats, sinks or swings wide when placed.',
        [{ op: 'origin-bottom' }]
      ),
    ],
    worst: 'warning',
    error: null,
    thumbnail: `${thumbs}/flower-shrubs.webp`,
    thumbnailState: 'ready',
    link: {
      job: 'flower-shrubs-1',
      project: 'moon-island',
      asset: 'flower-shrubs',
      assetName: 'Flower shrubs',
      provider: 'tripo',
    },
  };
  const spire: InboxItem = {
    id: 'far_spire.glb',
    name: 'Far spire',
    file: `${dir}/far_spire.glb`,
    bytes: 31e6,
    modifiedMs: now - 1 * MIN,
    kind: 'prop',
    kindGuessed: true,
    kindFromSend: false,
    state: 'failed',
    report: null,
    checks: [],
    worst: 'fail',
    error: 'This is not a glTF model. The file ends early; the download may still be running.',
    thumbnail: null,
    thumbnailState: 'fallback',
    link: null,
  };
  return [spire, character, sakura, shrubs];
}

/** Which bundled model (under `assets/mock/models`) previews each mock item. */
export const PREVIEW_FILE: Record<string, string> = {
  'character.glb': 'character.glb',
  'sakura_large.glb': 'sakura_tree_large.glb',
  'flower_shrubs.glb': 'flower_shrubs.glb',
};
