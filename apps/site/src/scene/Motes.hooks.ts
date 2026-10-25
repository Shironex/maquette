import { useFrame } from '@react-three/fiber';
import { useEffect, useMemo } from 'react';
import * as THREE from 'three';
import { LAMP_AIM, LAMP_BULB } from './Lights.hooks';
import { BENCH_TOP } from './manifest';

/** How many motes, and the box they drift in (metres): over the bench, out to where the camera flies. */
const COUNT = 220;
const BOX = { min: [-0.75, BENCH_TOP + 0.04, -0.5], max: [0.95, BENCH_TOP + 0.62, 0.72] } as const;
/** A mote's size in metres; it is drawn bigger the closer it is. */
const SIZE = 0.0022;

const vertexShader = /* glsl */ `
uniform float uTime, uScale, uSize;
uniform vec3 uBulb, uAxis;
attribute float aSeed;
varying float vLight;
void main() {
  vec3 p = position;
  float s = 0.6 + aSeed;
  p += vec3(
    sin(uTime * 0.11 * s + aSeed * 40.0),
    sin(uTime * 0.07 * s + aSeed * 23.0) * 0.7,
    cos(uTime * 0.09 * s + aSeed * 31.0)
  ) * 0.035;
  vec3 ray = p - uBulb;
  float cone = smoothstep(0.55, 0.93, dot(normalize(ray), uAxis));
  float twinkle = 0.55 + 0.45 * sin(uTime * (0.4 + aSeed) + aSeed * 60.0);
  vLight = cone * twinkle / (1.0 + dot(ray, ray) * 2.5);
  vec4 view = modelViewMatrix * vec4(p, 1.0);
  gl_Position = projectionMatrix * view;
  gl_PointSize = max(1.0, uSize * uScale * projectionMatrix[1][1] / -view.z);
}
`;

const fragmentShader = /* glsl */ `
varying float vLight;
void main() {
  float d = length(gl_PointCoord - 0.5);
  float a = smoothstep(0.5, 0.0, d) * vLight;
  if (a < 0.004) discard;
  gl_FragColor = vec4(vec3(1.0, 0.8, 0.6) * a, a);
}
`;

/**
 * Dust in the lamp's light: a few hundred points that drift on the GPU and
 * show only inside the lamp's cone, catching the light as they turn. One draw
 * call. Unless `still`, they move with the clock.
 */
export function useMotes(still: boolean) {
  const points = useMemo(() => {
    const positions = new Float32Array(COUNT * 3);
    const seeds = new Float32Array(COUNT);
    // A fixed sequence, so every load and every poster has the same dust.
    let seed = 7;
    const random = () => {
      seed = (seed * 16807) % 2147483647;
      return seed / 2147483647;
    };
    for (let i = 0; i < COUNT; i++) {
      for (let axis = 0; axis < 3; axis++) {
        const min = BOX.min[axis] ?? 0;
        const max = BOX.max[axis] ?? 0;
        positions[i * 3 + axis] = min + random() * (max - min);
      }
      seeds[i] = random();
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute('aSeed', new THREE.BufferAttribute(seeds, 1));
    const bulb = new THREE.Vector3(...LAMP_BULB);
    const material = new THREE.ShaderMaterial({
      vertexShader,
      fragmentShader,
      uniforms: {
        uTime: { value: 0 },
        uScale: { value: 900 },
        uSize: { value: SIZE },
        uBulb: { value: bulb },
        uAxis: { value: new THREE.Vector3(...LAMP_AIM).sub(bulb).normalize() },
      },
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    const cloud = new THREE.Points(geometry, material);
    cloud.frustumCulled = false;
    return cloud;
  }, []);

  useEffect(
    () => () => {
      points.geometry.dispose();
      points.material.dispose();
    },
    [points]
  );

  useFrame(state => {
    const uniforms = points.material.uniforms;
    if (uniforms.uTime) uniforms.uTime.value = still ? 0 : state.clock.elapsedTime;
    // Half the drawing buffer's height: the point size is in buffer pixels.
    if (uniforms.uScale) uniforms.uScale.value = (state.size.height * state.viewport.dpr) / 2;
  });

  return points;
}
