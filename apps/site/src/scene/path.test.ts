import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { BENCH_TOP, STOPS, type CameraStop } from './manifest';
import { MOVE, createPath, createShot, moved, type Shot } from './path';
import { LAST_STOP } from './progress';

const path = createPath(STOPS);
const shot = createShot();
const stopAt = (i: number) => STOPS[i] as CameraStop;

function expectOn(stop: CameraStop, at: Shot = shot) {
  expect(at.position.distanceTo(new THREE.Vector3(...stop.position))).toBeLessThan(1e-5);
  expect(at.target.distanceTo(new THREE.Vector3(...stop.target))).toBeLessThan(1e-5);
  expect(at.fov).toBeCloseTo(stop.fov);
}

describe('moved', () => {
  const base: CameraStop = { id: 'concept', position: [0, 1, 1], target: [0, 1, 0], fov: 30 };

  it('is the stop itself at the middle of the move', () => {
    expectOn(base, moved({ ...base, move: { dolly: 0.2, rise: 0.1, orbit: 20 } }, 0));
  });

  it('dollies toward the target and rises', () => {
    const { position } = moved({ ...base, move: { dolly: 0.2, rise: 0.1 } }, 0.5);
    expect(position.y).toBeCloseTo(1.05);
    expect(position.z).toBeCloseTo(0.9);
  });

  it('trucks the camera and the target together', () => {
    const { position, target } = moved({ ...base, move: { truck: [-0.2, 0, 0] } }, 0.5);
    expect(position.x).toBeCloseTo(-0.1);
    expect(target.x).toBeCloseTo(-0.1);
  });

  it('orbits about the pivot at a constant distance', () => {
    const { position } = moved({ ...base, move: { orbit: 90 } }, 1);
    expect(position.x).toBeCloseTo(1);
    expect(position.z).toBeCloseTo(0);
  });
});

describe('camera path', () => {
  it('shows each stop framing at its own progress', () => {
    STOPS.forEach((stop, i) => {
      path.shotAt(i, shot);
      expectOn(stop);
    });
  });

  it('plays each stop move across its window', () => {
    const model = stopAt(3);
    for (const [offset, t] of [
      [-MOVE, -0.5],
      [MOVE, 0.5],
    ] as const) {
      path.shotAt(3 + offset, shot);
      expect(shot.position.distanceTo(moved(model, t).position)).toBeLessThan(1e-5);
    }
  });

  it('starts on the hero frame and clamps outside the tour', () => {
    path.shotAt(-2, shot);
    expectOn(stopAt(0));
    path.shotAt(LAST_STOP + 2, shot);
    expectOn(stopAt(LAST_STOP));
  });

  it('cuts to the nearest stop for poster capture', () => {
    path.shotAt(2.4, shot, true);
    expectOn(stopAt(2));
    path.shotAt(2.6, shot, true);
    expectOn(stopAt(3));
  });

  it('flies a long way between stops: across the bench, not centimetres', () => {
    const step = 0.01;
    const previous = createShot();
    for (let stop = 0; stop < LAST_STOP; stop++) {
      let travelled = 0;
      path.shotAt(stop, previous);
      for (let p = stop + step; p <= stop + 1 + 1e-9; p += step) {
        path.shotAt(p, shot);
        travelled += shot.position.distanceTo(previous.position);
        previous.position.copy(shot.position);
      }
      expect(travelled).toBeGreaterThan(0.4);
    }
  });

  it('never stops between stops, and moves slowest through them', () => {
    const step = 0.01;
    const speed = (p: number) => {
      path.shotAt(p - step / 2, shot);
      const from = shot.position.clone();
      path.shotAt(p + step / 2, shot);
      return shot.position.distanceTo(from) / step;
    };
    for (let stop = 1; stop < LAST_STOP; stop++) {
      expect(speed(stop)).toBeGreaterThan(0.02);
      let flight = 0;
      for (let p = stop + 0.25; p <= stop + 0.75; p += 0.05) flight = Math.max(flight, speed(p));
      expect(flight).toBeGreaterThan(speed(stop) * 1.5);
    }
  });

  it('moves continuously, without jumps between samples', () => {
    const step = 0.005;
    const previous = createShot();
    path.shotAt(0, previous);
    for (let p = step; p <= LAST_STOP + 1e-9; p += step) {
      path.shotAt(p, shot);
      expect(shot.position.distanceTo(previous.position)).toBeLessThan(0.06);
      expect(shot.target.distanceTo(previous.target)).toBeLessThan(0.08);
      expect(Math.abs(shot.fov - previous.fov)).toBeLessThan(0.5);
      previous.position.copy(shot.position);
      previous.target.copy(shot.target);
      previous.fov = shot.fov;
    }
  });

  it('stays above the bench, in front of the wall and clear of the tall props', () => {
    // The clay island, the finished island and the lamp, as spheres.
    const props = [
      { at: new THREE.Vector3(-0.02, 1.14, -0.04), radius: 0.28 },
      { at: new THREE.Vector3(0.44, 1.19, -0.08), radius: 0.28 },
      { at: new THREE.Vector3(0.7, 1.15, -0.29), radius: 0.3 },
    ];
    for (let p = 0; p <= LAST_STOP; p += 0.01) {
      path.shotAt(p, shot);
      expect(shot.position.y).toBeGreaterThan(BENCH_TOP + 0.1);
      expect(shot.position.z).toBeGreaterThan(0.2);
      for (const prop of props)
        expect(shot.position.distanceTo(prop.at)).toBeGreaterThan(prop.radius);
    }
  });
});
