/**
 * Builds a path (./paths.ts) into one tube mesh: a single draw call whose
 * shader lights it from the first point by the `uAmount` uniform, with the
 * same front and head as `litAt` and `headAt`. The tube's U runs along its
 * length, so the fill follows the distance along the path.
 */
import * as THREE from 'three';
import { HEAD, SOFT, TUBE_SIDES, tubeSegments, type ScenePath } from './paths';

const vertexShader = /* glsl */ `
varying float vU;
varying vec3 vNormal;
void main() {
  vU = uv.x;
  vNormal = normalize(normalMatrix * normal);
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

const fragmentShader = /* glsl */ `
uniform vec3 uColor, uGlow;
uniform float uAmount;
varying float vU;
varying vec3 vNormal;
const float SOFT = ${SOFT.toFixed(4)};
const float HEAD = ${HEAD.toFixed(4)};
void main() {
  float a = clamp(uAmount, 0.0, 1.0);
  float front = a * (1.0 + SOFT);
  float lit = clamp((front - vU) / SOFT, 0.0, 1.0);
  float filling = step(0.0001, a) * step(a, 0.9999);
  float head = filling * clamp(1.0 - abs(vU - (front - SOFT)) / HEAD, 0.0, 1.0);
  float shade = 0.45 + 0.55 * max(dot(normalize(vNormal), normalize(vec3(0.3, 1.0, 0.5))), 0.0);
  vec3 cable = uColor * shade * 0.6;
  vec3 color = mix(cable, uGlow * 1.25, lit) + uGlow * head * 0.9;
  gl_FragColor = vec4(color, 1.0);
  #include <colorspace_fragment>
}
`;

/** The curve the tube follows: straight runs, or a smooth curve through every point. */
export function curveOf(path: ScenePath): THREE.Curve<THREE.Vector3> {
  const points = path.points.map(p => new THREE.Vector3(...p));
  if (path.curve === 'catmull-rom') return new THREE.CatmullRomCurve3(points, false, 'centripetal');
  const runs = new THREE.CurvePath<THREE.Vector3>();
  for (let i = 1; i < points.length; i += 1) {
    runs.add(new THREE.LineCurve3(points[i - 1], points[i]));
  }
  return runs;
}

export type PathMesh = THREE.Mesh<THREE.TubeGeometry, THREE.ShaderMaterial>;

/** The path's tube, unlit; set `material.uniforms.uAmount` to energize it. */
export function buildPath(path: ScenePath): PathMesh {
  const geometry = new THREE.TubeGeometry(
    curveOf(path),
    tubeSegments(path),
    path.width / 2,
    TUBE_SIDES,
    false
  );
  const material = new THREE.ShaderMaterial({
    vertexShader,
    fragmentShader,
    uniforms: {
      uColor: { value: new THREE.Color(path.color) },
      uGlow: { value: new THREE.Color(path.glow) },
      uAmount: { value: 0 },
    },
  });
  const mesh = new THREE.Mesh(geometry, material);
  mesh.name = `path-${path.id}`;
  return mesh;
}

/** Sets how much of a built path is lit. */
export function energize(mesh: PathMesh, amount: number): void {
  const uniform = mesh.material.uniforms.uAmount;
  if (uniform) uniform.value = amount;
}
