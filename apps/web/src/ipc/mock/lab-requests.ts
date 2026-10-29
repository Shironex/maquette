/**
 * The fake Lab's start requests: a brief and a concept's objects, checked
 * like the core checks them, saved as a direction and queued.
 */
import type { Direction, LabView, NewConcepts, NewObjectRefs } from '../client';
import { clock } from '@/lib/clock';
import { objectsDirectionName, slug } from './lab-helpers';

export interface LabRequestsDeps {
  fail: (code: string, message: string) => never;
  list: (project: string) => Direction[];
  saveDirection: (project: string, next: Direction) => Direction;
  /** The style board fields a new direction carries. */
  stamp: (
    project: string,
    useStyle: boolean | undefined
  ) => Pick<Direction, 'styleOff' | 'styleRevision'>;
  enqueue: (project: string, direction: Direction, titles: (string | null)[]) => LabView;
}

export function createLabRequests({ fail, list, saveDirection, stamp, enqueue }: LabRequestsDeps) {
  function generate(request: NewConcepts) {
    const name = request.direction.trim();
    if (!name) fail('invalid', 'Name the direction.');
    if (!request.prompt.trim()) fail('invalid', 'Describe what the concepts should show.');
    if (request.count < 1 || request.count > 12)
      fail('invalid', 'Ask for between 1 and 12 images.');
    const id = slug(name);
    const at = clock.now().toISOString();
    const existing = list(request.project).find(d => d.id === id);
    const direction = saveDirection(request.project, {
      id,
      name,
      prompt: request.prompt.trim(),
      refs: request.refs,
      aspect: request.aspect,
      images: existing?.images ?? [],
      createdAt: existing?.createdAt ?? at,
      updatedAt: at,
      source: existing?.source ?? null,
      kind: existing?.kind ?? null,
      ...stamp(request.project, request.useStyle),
    });
    return enqueue(request.project, direction, Array<string | null>(request.count).fill(null));
  }

  /** Mirrors `concepts::objects`: one titled job per object, the concept attached. */
  function makeObjects(request: NewObjectRefs) {
    const names = request.objects.map(o => o.trim()).filter(Boolean);
    if (names.length === 0) fail('invalid', 'Name at least one object.');
    if (names.length > 12) fail('invalid', 'Make 12 object refs at most in one go.');
    const long = names.find(n => n.length > 60);
    if (long) fail('invalid', `Keep each object to 60 characters or fewer: "${long}" is longer.`);
    if (request.kind === 'scene')
      fail('invalid', 'Object refs are props or characters, not scenes.');
    const base = request.sourceName.trim() || 'Concept';
    const name = objectsDirectionName(list(request.project), request);
    const id = slug(name);
    const at = clock.now().toISOString();
    const existing = list(request.project).find(d => d.id === id);
    const direction = saveDirection(request.project, {
      id,
      name,
      prompt: `One clean reference image of each object in ${base}.`,
      refs: [request.source],
      aspect: request.aspect,
      images: existing?.images ?? [],
      createdAt: existing?.createdAt ?? at,
      updatedAt: at,
      source: request.source,
      kind: request.kind,
      ...stamp(request.project, request.useStyle),
    });
    return enqueue(request.project, direction, names);
  }

  return { generate, makeObjects };
}
