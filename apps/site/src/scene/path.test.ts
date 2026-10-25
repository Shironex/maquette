import { describe, expect, it } from 'vitest';
import { BENCH_TOP, STOPS, type CameraStop } from './manifest';
import { MOVE, createPath, createShot, moved } from './path';
import { LAST_STOP } from './progress';

const path = createPath(STOPS);
const shot = createShot();
const stopAt = (i: number) => STOPS[i] as CameraStop;

function expectOn(stop: CameraStop) {
  expect(
    shot.position.distanceTo({ x: stop.position[0], y: stop.position[1], z: stop.position[2] })
  ).toBeLessThan(1e-6);
  expect(
    shot.target.distanceTo({ x: stop.target[0], y: stop.target[1], z: stop.target[2] })
  ).toBeLessThan(1e-6);
  expect(shot.fov).toBeCloseTo(stop.fov);
}

describe('moved', () => {
  const base: CameraStop = { id: 'concept', position: [0, 1, 1], target: [0, 1, 0], fov: 30 };

  it('is the stop itself at the middle of the move', () => {
    expect(moved({ ...base, move: { dolly: 0.2, rise: 0.1, orbit: 20 } }, 0)).toEqual([
      0, 1, 1, 0, 1, 0, 30,
    ]);
  });

  it('dollies toward the target and rises', () => {
    const [x, y, z] = moved({ ...base, move: { dolly: 0.2, rise: 0.1 } }, 0.5);
    expect([x, y, z].map(v => +(v ?? 0).toFixed(6))).toEqual([0, 1.05, 0.9]);
  });

  it('trucks the camera and the target together', () => {
    const value = moved({ ...base, move: { truck: [-0.2, 0, 0] } }, 0.5);
    expect(value[0]).toBeCloseTo(-0.1);
    expect(value[3]).toBeCloseTo(-0.1);
  });

  it('orbits about the pivot at a constant distance', () => {
    const [x = 0, , z = 0] = moved({ ...base, move: { orbit: 90 } }, 1);
    expect(x).toBeCloseTo(1);
    expect(z).toBeCloseTo(0);
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
    path.shotAt(3 - MOVE, shot);
    const start = moved(model, -0.5);
    expect(shot.position.x).toBeCloseTo(start[0] ?? 0);
    path.shotAt(3 + MOVE, shot);
    const end = moved(model, 0.5);
    expect(shot.position.x).toBeCloseTo(end[0] ?? 0);
  });

  it('holds still on a stop without a move', () => {
    const blender = stopAt(4);
    for (const p of [4 - MOVE, 4 - MOVE / 2, 4 + MOVE]) {
      path.shotAt(p, shot);
      expectOn(blender);
    }
  });

  it('ends the tour on the last stop and clamps outside it', () => {
    path.shotAt(-2, shot);
    expectOn(stopAt(0));
    path.shotAt(LAST_STOP + 2, shot);
    expectOn(stopAt(LAST_STOP));
  });

  it('cuts to the nearest stop with reduced motion', () => {
    path.shotAt(2.4, shot, true);
    expectOn(stopAt(2));
    path.shotAt(2.6, shot, true);
    expectOn(stopAt(3));
  });

  it('moves continuously, without jumps between samples', () => {
    const step = 0.005;
    const previous = createShot();
    path.shotAt(0, previous);
    for (let p = step; p <= LAST_STOP + 1e-9; p += step) {
      path.shotAt(p, shot);
      expect(shot.position.distanceTo(previous.position)).toBeLessThan(0.03);
      expect(shot.target.distanceTo(previous.target)).toBeLessThan(0.03);
      expect(Math.abs(shot.fov - previous.fov)).toBeLessThan(0.5);
      previous.position.copy(shot.position);
      previous.target.copy(shot.target);
      previous.fov = shot.fov;
    }
  });

  it('stays above the bench and in front of the props on the way', () => {
    for (let p = 0; p <= LAST_STOP; p += 0.01) {
      path.shotAt(p, shot);
      expect(shot.position.y).toBeGreaterThan(BENCH_TOP + 0.2);
      // The props stand behind z = 0.4; the cloth at the front is low.
      expect(shot.position.z).toBeGreaterThan(0.5);
    }
  });
});
