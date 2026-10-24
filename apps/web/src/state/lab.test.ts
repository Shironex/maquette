import { describe, expect, it } from 'vitest';
import type { ConceptJob, Direction, LabView } from '@/ipc/client';
import {
  MAX_OBJECTS,
  addObjects,
  applyConceptEvent,
  coverImage,
  directionId,
  directionSummary,
  formatElapsed,
  inProject,
  jobMeta,
  queueFooter,
  queueSummary,
  objectsDirectionName,
  tileLabel,
  tiles,
} from './lab';

function image(index: number, marks: { starred?: boolean; picked?: boolean } = {}) {
  return {
    file: `concepts/porch/porch_${String(index).padStart(2, '0')}.png`,
    index,
    width: 1672,
    height: 941,
    starred: marks.starred ?? false,
    picked: marks.picked ?? false,
    refFile: null,
    createdAt: '2026-10-24T10:00:00Z',
    title: null,
  };
}

const direction: Direction = {
  id: 'porch',
  name: 'Porch',
  prompt: 'A porch',
  refs: [],
  aspect: '16:9',
  images: [image(2, { starred: true }), image(1, { starred: true, picked: true })],
  createdAt: '2026-10-24T10:00:00Z',
  updatedAt: '2026-10-24T10:00:00Z',
  source: null,
  kind: null,
};

function job(
  index: number,
  state: ConceptJob['state'],
  extra: Partial<ConceptJob> = {}
): ConceptJob {
  return {
    id: `porch-${index}`,
    runId: null,
    project: 'moon',
    direction: 'porch',
    directionName: 'Porch',
    title: null,
    index,
    file: `concepts/porch/porch_${String(index).padStart(2, '0')}.png`,
    state,
    queuedAtMs: 0,
    startedAtMs: null,
    finishedAtMs: null,
    message: null,
    width: null,
    height: null,
    logTail: [],
    ...extra,
  };
}

describe('lab helpers', () => {
  it('orders images first, then the jobs still going', () => {
    const t = tiles(direction, [
      job(3, 'running'),
      job(1, 'done'),
      job(4, 'failed'),
      job(5, 'queued'),
    ]);
    expect(t.map(x => x.key)).toEqual([
      'concepts/porch/porch_01.png',
      'concepts/porch/porch_02.png',
      'porch-3',
      'porch-5',
    ]);
  });

  it('summarises a direction and the queue', () => {
    expect(directionSummary(direction, [job(3, 'running')])).toBe(
      '3 concepts · 2 starred · 1 picked'
    );
    const s = queueSummary([job(1, 'done'), job(2, 'running'), job(3, 'queued'), job(4, 'failed')]);
    expect(s).toEqual({ done: 1, total: 4, running: 1, queued: 1, failed: 1 });
    expect(queueFooter(s, 4)).toBe('1 running, 1 waiting, 4 at a time');
    expect(queueFooter(queueSummary([]), 4)).toBe('Idle, 4 at a time');
  });

  it('describes each job the way the queue shows it', () => {
    expect(formatElapsed(58_000)).toBe('58s');
    expect(formatElapsed(64_000)).toBe('1m 04s');
    expect(jobMeta(job(1, 'running', { startedAtMs: 1000 }), 43_000)).toBe('Generating · 42s');
    expect(
      jobMeta(job(1, 'done', { startedAtMs: 0, finishedAtMs: 64_000, width: 1672, height: 941 }), 0)
    ).toBe('1672 × 941 · 1m 04s');
    expect(jobMeta(job(1, 'failed', { message: 'Codex stopped.' }), 0)).toBe('Codex stopped.');
  });

  it('folds events into the view of the same project only', () => {
    const view: LabView = {
      project: 'moon',
      projectDir: '/w/projects/moon',
      directions: [direction],
      jobs: [job(3, 'queued')],
      refs: [],
      concurrency: 4,
    };
    const running = applyConceptEvent(view, { type: 'job', job: job(3, 'running') });
    expect(running.jobs[0]?.state).toBe('running');
    const added = applyConceptEvent(running, { type: 'job', job: job(4, 'queued') });
    expect(added.jobs).toHaveLength(2);
    const other = applyConceptEvent(view, {
      type: 'job',
      job: { ...job(9, 'queued'), project: 'x' },
    });
    expect(other).toBe(view);
    const next = { ...direction, images: [...direction.images, image(3)] };
    const updated = applyConceptEvent(view, {
      type: 'direction',
      project: 'moon',
      direction: next,
    });
    expect(updated.directions[0]?.images).toHaveLength(3);
  });

  it('picks a cover and builds paths', () => {
    expect(coverImage(direction)?.index).toBe(1);
    expect(directionId('Moon Island porch!')).toBe('moon-island-porch');
    expect(inProject('/w/moon/', 'concepts/a.png')).toBe('/w/moon/concepts/a.png');
    expect(inProject('C:\\w\\moon', 'concepts/a.png')).toBe('C:\\w\\moon\\concepts\\a.png');
    expect(inProject('/w/moon', '/abs/b.png')).toBe('/abs/b.png');
  });
});

describe('object refs', () => {
  it('adds typed and pasted objects once each, capitalised', () => {
    expect(addObjects([], 'desk lamp')).toEqual(['Desk lamp']);
    expect(addObjects(['Desk lamp'], 'brush  cup, DESK LAMP;\ncalipers\n\n')).toEqual([
      'Desk lamp',
      'Brush cup',
      'Calipers',
    ]);
    expect(addObjects([], 'x'.repeat(80))[0]).toHaveLength(60);
    const many = Array.from({ length: 20 }, (_, i) => `thing ${i}`).join(',');
    expect(addObjects([], many)).toHaveLength(MAX_OBJECTS);
  });

  it('names the direction and the tiles after the objects', () => {
    expect(objectsDirectionName('Workbench hero')).toBe('Workbench hero objects');
    expect(objectsDirectionName('  ')).toBe('Concept objects');
    expect(objectsDirectionName('x'.repeat(70))).toHaveLength(60);
    expect(tileLabel({ index: 3, title: 'Desk lamp' })).toBe('Desk lamp');
    expect(tileLabel({ index: 3, title: null })).toBe('Concept 03');
  });

  it('counts objects, not concepts, in an objects direction', () => {
    const objects: Direction = {
      ...direction,
      source: 'concepts/porch/porch_01.png',
      kind: 'prop',
    };
    expect(directionSummary(objects, [])).toMatch(/^2 objects/);
  });
});
