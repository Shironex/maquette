import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import type { BuiltModel } from './build';
import type { ModelEntry } from './manifest';
import { meshParts, rigTracks, type TimelineTracks } from './tracks';

const STOPS = ['porch', 'sakura', 'shrubs'];

function part(name: string, x: number): THREE.Mesh {
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(0.2, 0.2, 0.2));
  mesh.name = name.replace(/\s/g, '_');
  mesh.userData.name = name;
  mesh.position.x = x;
  return mesh;
}

/** A cabinet as the loader gives it: a body, and a door to its right. */
function cabinet(): BuiltModel {
  const model = new THREE.Group();
  model.add(part('Body', 0), part('Front door', 0.5));
  const root = new THREE.Group();
  root.add(model);
  const entry = {
    id: 'cabinet',
    file: 'cabinet.glb',
    position: [0, 0, 0],
    anchor: 'origin',
    stops: ['hero'],
  };
  return { entry: entry as ModelEntry, root, model, sweep: null };
}

const timeline = (overrides: Partial<TimelineTracks> = {}): TimelineTracks => ({
  stopIds: STOPS,
  states: {},
  explode: {},
  ...overrides,
});

describe('the scroll timeline tracks', () => {
  it('finds the nodes that draw a mesh', () => {
    const built = cabinet();
    expect(meshParts(built.model).map(n => n.userData.name)).toEqual(['Body', 'Front door']);
  });

  it('leaves a model the timeline does not move alone', () => {
    expect(rigTracks(cabinet(), timeline())).toBeNull();
  });

  it('spreads the parts out from the centre by the keyed amount', () => {
    const built = cabinet();
    const tracks = rigTracks(
      built,
      timeline({
        explode: {
          cabinet: {
            distance: 1,
            easing: 'linear',
            keys: [
              { stop: 'porch', amount: 0 },
              { stop: 'shrubs', amount: 1 },
            ],
          },
        },
      })
    )!;
    const door = built.model.getObjectByName('Front_door')!;
    const body = built.model.getObjectByName('Body')!;
    tracks.apply(1, 0);
    expect(door.position.x).toBeCloseTo(1);
    expect(body.position.x).toBeCloseTo(-0.5);
    tracks.apply(0, 0);
    expect(door.position.x).toBeCloseTo(0.5);
  });

  it('turns a part into its state once the camera reaches the stop', () => {
    const built = cabinet();
    const tracks = rigTracks(
      built,
      timeline({
        states: {
          cabinet: {
            states: [
              { id: 'closed', name: 'Closed', parts: {} },
              {
                id: 'open',
                name: 'Open',
                parts: { 'Front door': { position: [0, 0, 0], rotation: [0, -90, 0] } },
              },
            ],
            transitions: [],
            triggers: [
              { kind: 'stop', to: 'open', stop: 'sakura', part: null, start: null, end: null },
            ],
          },
        },
      })
    )!;
    const door = built.model.getObjectByName('Front_door')!;
    expect(tracks.apply(0, 0)).toBe(false);
    expect(tracks.apply(1, 1)).toBe(true);
    tracks.apply(1, 2);
    expect(door.rotation.y).toBeCloseTo(-Math.PI / 2);
  });
});
