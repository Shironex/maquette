/**
 * The studio every model preview stands in: a graphite backdrop, a soft
 * floor that fades into it with a contact shadow, a warm key, a warm fill
 * and a cool rim light, and a three quarter camera framed to the model.
 * Plain three.js, shared by the live preview and the offscreen snapshot, and
 * matched to the Blender thumbnail render so the two read as one look.
 */
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { KTX2Loader } from 'three/examples/jsm/loaders/KTX2Loader.js';
import { MeshoptDecoder } from 'three/examples/jsm/libs/meshopt_decoder.module.js';

/** The kit's studio token, also the Blender render's backdrop. */
export const BACKDROP = '#312f2b';
export const CLAY = '#d08c6f';
const AZIMUTH = THREE.MathUtils.degToRad(38);
const ELEVATION = THREE.MathUtils.degToRad(20);
/** How much of the frame the model fills. */
const FILL = 0.78;

/** Where three's Basis transcoder lives, copied into the web app's public folder. */
const TRANSCODER = `${import.meta.env.BASE_URL}basis/`;

let ktx2: KTX2Loader | null = null;

/**
 * A glTF loader that reads meshopt geometry and KTX2 textures. The KTX2
 * loader is shared (it keeps a worker pool) and needs a renderer once, to
 * learn which compressed formats the GPU takes.
 */
export function gltfLoader(renderer: THREE.WebGLRenderer): GLTFLoader {
  if (!ktx2) {
    ktx2 = new KTX2Loader().setTranscoderPath(TRANSCODER).detectSupport(renderer);
  }
  return new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).setKTX2Loader(ktx2);
}

function radialTexture(stops: readonly [number, string][]): THREE.CanvasTexture {
  const size = 256;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');
  if (ctx) {
    const g = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
    for (const [at, color] of stops) g.addColorStop(at, color);
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, size, size);
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function disc(size: number, material: THREE.Material, y: number): THREE.Mesh {
  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(size, size), material);
  mesh.rotation.x = -Math.PI / 2;
  mesh.position.y = y;
  return mesh;
}

export interface Framing {
  /** The model's bounds after it was stood on the floor at the origin. */
  box: THREE.Box3;
  /** The longest diagonal of the bounds. */
  extent: number;
  target: THREE.Vector3;
}

/**
 * The meshes' bounds, or the bones' for a skeleton with no mesh (a library
 * clip), so a bare skeleton frames too.
 */
function boundsOf(object: THREE.Object3D): THREE.Box3 {
  const box = new THREE.Box3().setFromObject(object);
  if (!box.isEmpty()) return box;
  const at = new THREE.Vector3();
  object.traverse(o => {
    if ((o as THREE.Bone).isBone) box.expandByPoint(o.getWorldPosition(at));
  });
  // Room for the limbs to swing and the root to travel.
  return box.isEmpty() ? box : box.expandByScalar(box.getSize(at).length() * 0.25);
}

/** Moves `object` so it stands on the floor (y = 0), centred on the origin. */
export function standOnFloor(object: THREE.Object3D): Framing | null {
  object.updateMatrixWorld(true);
  const box = boundsOf(object);
  if (box.isEmpty()) return null;
  const center = box.getCenter(new THREE.Vector3());
  object.position.sub(new THREE.Vector3(center.x, box.min.y, center.z));
  object.updateMatrixWorld(true);
  const placed = boundsOf(object);
  const extent = placed.getSize(new THREE.Vector3()).length() || 1;
  return { box: placed, extent, target: placed.getCenter(new THREE.Vector3()) };
}

/**
 * Puts `camera` at three quarters (front right, a little above) far enough
 * back that every corner of the bounds fits the frame.
 */
export function frameCamera(camera: THREE.PerspectiveCamera, framing: Framing): void {
  const v = new THREE.Vector3(
    Math.sin(AZIMUTH) * Math.cos(ELEVATION),
    Math.sin(ELEVATION),
    Math.cos(AZIMUTH) * Math.cos(ELEVATION)
  ).normalize();
  const right = new THREE.Vector3().crossVectors(new THREE.Vector3(0, 1, 0), v).normalize();
  const up = new THREE.Vector3().crossVectors(v, right).normalize();
  const tanV = Math.tan(THREE.MathUtils.degToRad(camera.fov / 2));
  const tanH = tanV * (camera.aspect || 1);
  const { box, target } = framing;
  let distance = 0;
  for (const x of [box.min.x, box.max.x]) {
    for (const y of [box.min.y, box.max.y]) {
      for (const z of [box.min.z, box.max.z]) {
        const q = new THREE.Vector3(x, y, z).sub(target);
        const across = Math.abs(q.dot(right)) / (tanH * FILL);
        const tall = Math.abs(q.dot(up)) / (tanV * FILL);
        distance = Math.max(distance, Math.max(across, tall) + q.dot(v));
      }
    }
  }
  distance = Math.max(distance, 0.01);
  camera.position.copy(target).addScaledVector(v, distance);
  camera.near = distance / 100;
  camera.far = distance * 100;
  camera.lookAt(target);
  camera.updateProjectionMatrix();
}

/**
 * The lights, floor and shadow for a model of `framing`'s size, in one
 * group, so the preview can swap it when the model changes.
 */
export function studioRig(framing: Framing): THREE.Group {
  const rig = new THREE.Group();
  rig.name = 'studio';
  const { extent, target } = framing;
  const size = framing.box.getSize(new THREE.Vector3());

  rig.add(new THREE.HemisphereLight('#f4efe6', '#2a2926', 0.7));

  const key = new THREE.DirectionalLight('#ffd9b5', 2.6);
  key.position.set(-extent * 0.9, extent * 1.4, extent * 1.2).add(target);
  key.target.position.copy(target);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  key.shadow.radius = 4;
  const reach = extent * 0.9;
  const cam = key.shadow.camera;
  cam.left = -reach;
  cam.right = reach;
  cam.top = reach;
  cam.bottom = -reach;
  cam.near = extent * 0.1;
  cam.far = extent * 4;
  cam.updateProjectionMatrix();
  key.shadow.bias = -0.0005;
  key.shadow.normalBias = extent * 0.002;
  rig.add(key, key.target);

  const fill = new THREE.DirectionalLight('#ffe9d6', 0.8);
  fill.position.set(extent * 1.4, extent * 0.4, extent * 0.6).add(target);
  fill.target.position.copy(target);
  rig.add(fill, fill.target);

  const rim = new THREE.DirectionalLight('#b8ccff', 1.8);
  rim.position.set(extent * 0.2, extent * 1.1, -extent * 1.6).add(target);
  rim.target.position.copy(target);
  rig.add(rim, rim.target);

  // The floor glow fades into the backdrop, so there is no horizon line.
  const glow = radialTexture([
    [0, 'rgba(255, 236, 214, 0.10)'],
    [0.45, 'rgba(255, 236, 214, 0.05)'],
    [1, 'rgba(255, 236, 214, 0)'],
  ]);
  rig.add(
    disc(
      extent * 5,
      new THREE.MeshBasicMaterial({ map: glow, transparent: true, depthWrite: false }),
      0
    )
  );
  // A soft contact blob right under the model, plus the key light's shadow.
  const footprint = Math.max(size.x, size.z) * 1.35 || extent;
  const blob = radialTexture([
    [0, 'rgba(10, 9, 8, 0.55)'],
    [0.6, 'rgba(10, 9, 8, 0.18)'],
    [1, 'rgba(10, 9, 8, 0)'],
  ]);
  const contact = disc(
    footprint,
    new THREE.MeshBasicMaterial({ map: blob, transparent: true, depthWrite: false }),
    extent * 0.0005
  );
  contact.position.x = target.x;
  contact.position.z = target.z;
  rig.add(contact);
  const shadow = disc(
    extent * 3,
    new THREE.ShadowMaterial({ opacity: 0.32, depthWrite: false }),
    extent * 0.001
  );
  shadow.receiveShadow = true;
  rig.add(shadow);
  return rig;
}

/** Lets every mesh of the model cast the key light's shadow. */
export function castShadows(object: THREE.Object3D): void {
  object.traverse(o => {
    if ((o as THREE.Mesh).isMesh) o.castShadow = true;
  });
}
