/**
 * The looks a loaded model can take in a preview: its own shaded materials,
 * a wireframe over flat clay, a UV checker, a ghost for the skeleton view,
 * or only its baked lightmap. Plain three.js, so the Inbox preview and the
 * Inspector's compare view share it.
 */
import * as THREE from 'three';

export type MaterialMode = 'shaded' | 'wireframe' | 'uv' | 'bones' | 'lightmap';

/** An 8 by 8 checker with a number per tile, so stretched UVs show. */
export function checkerTexture(): THREE.CanvasTexture {
  const size = 512;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');
  if (ctx) {
    const tile = size / 8;
    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        ctx.fillStyle = (x + y) % 2 === 0 ? '#d9cbb8' : '#5d564d';
        ctx.fillRect(x * tile, y * tile, tile, tile);
        ctx.fillStyle = (x + y) % 2 === 0 ? '#5d564d' : '#d9cbb8';
        ctx.font = '600 20px sans-serif';
        ctx.fillText(`${String.fromCharCode(65 + x)}${y + 1}`, x * tile + 8, y * tile + 26);
      }
    }
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  return texture;
}

export type Originals = Map<THREE.Mesh, THREE.Material | THREE.Material[]>;

/** Every mesh of `scene` with the material it loaded with. */
export function originalsOf(scene: THREE.Object3D): Originals {
  const originals: Originals = new Map();
  scene.traverse(obj => {
    if (!(obj as THREE.Mesh).isMesh) return;
    const mesh = obj as THREE.Mesh;
    originals.set(mesh, mesh.material);
  });
  return originals;
}

/** A copy of `mesh` drawn with `material`, deforming with it if it is skinned. */
function overlayOf(mesh: THREE.Mesh, material: THREE.Material): THREE.Mesh {
  if ((mesh as THREE.SkinnedMesh).isSkinnedMesh) {
    const skinned = mesh as THREE.SkinnedMesh;
    const copy = new THREE.SkinnedMesh(skinned.geometry, material);
    copy.bindMode = skinned.bindMode;
    copy.bind(skinned.skeleton, skinned.bindMatrix);
    return copy;
  }
  return new THREE.Mesh(mesh.geometry, material);
}

/**
 * The lightmap a material carries: the pipeline's export stores the baked
 * light as the emissive map on the second UV set and marks the material's
 * extras with `lightmap`.
 */
export function lightmapOf(material: THREE.Material): THREE.Texture | null {
  const m = material as THREE.MeshStandardMaterial;
  const map = m.emissiveMap ?? m.lightMap ?? null;
  if (!map) return null;
  return m.userData?.lightmap || map.channel === 1 || m.lightMap ? map : null;
}

function first(material: THREE.Material | THREE.Material[]): THREE.Material | undefined {
  return Array.isArray(material) ? material[0] : material;
}

export interface Applied {
  /** Puts the original materials back and frees the ones made here. */
  undo: () => void;
  /** Lightmap mode only: how many meshes had a baked lightmap. */
  lightmapped: number;
}

/** Gives every mesh in `originals` the look of `mode`. */
export function applyMode(
  originals: Originals,
  mode: MaterialMode,
  checker: THREE.Texture
): Applied {
  const made: THREE.Material[] = [];
  const overlays: THREE.Mesh[] = [];
  const keep = <T extends THREE.Material>(m: T): T => {
    made.push(m);
    return m;
  };
  // Wireframe: flat clay pushed back a little, with the edges drawn over it.
  const clay =
    mode === 'wireframe' || mode === 'lightmap'
      ? keep(
          new THREE.MeshStandardMaterial({
            color: mode === 'lightmap' ? '#4a463f' : '#6b645a',
            roughness: 0.95,
            polygonOffset: true,
            polygonOffsetFactor: 1,
            polygonOffsetUnits: 1,
          })
        )
      : null;
  const wire =
    mode === 'wireframe'
      ? keep(
          new THREE.MeshBasicMaterial({
            color: '#f1e4d6',
            wireframe: true,
            transparent: true,
            opacity: 0.38,
          })
        )
      : null;
  const ghost =
    mode === 'bones'
      ? keep(
          new THREE.MeshStandardMaterial({
            color: '#a39d93',
            roughness: 0.9,
            transparent: true,
            opacity: 0.22,
            depthWrite: false,
          })
        )
      : null;
  const uv =
    mode === 'uv' ? keep(new THREE.MeshStandardMaterial({ map: checker, roughness: 0.9 })) : null;
  const baked = new Map<THREE.Texture, THREE.Material>();
  let lightmapped = 0;
  for (const [mesh, original] of originals) {
    let replacement: THREE.Material | null = clay ?? ghost ?? uv;
    if (mode === 'lightmap') {
      const source = first(original);
      const map = source ? lightmapOf(source) : null;
      if (map) {
        lightmapped += 1;
        replacement =
          baked.get(map) ??
          keep(new THREE.MeshBasicMaterial({ map, side: source?.side ?? THREE.FrontSide }));
        baked.set(map, replacement);
      }
    }
    mesh.material = replacement ?? original;
    mesh.castShadow = mode !== 'bones';
    if (wire) {
      const overlay = overlayOf(mesh, wire);
      overlay.castShadow = false;
      mesh.add(overlay);
      overlays.push(overlay);
    }
  }
  return {
    lightmapped,
    undo: () => {
      for (const overlay of overlays) overlay.removeFromParent();
      for (const [mesh, original] of originals) mesh.material = original;
      made.forEach(m => m.dispose());
    },
  };
}
