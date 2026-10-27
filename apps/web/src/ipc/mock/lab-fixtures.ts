/**
 * The Frostveil concept directions mock mode shows. The images are the
 * bundled `assets/mock/concepts/*.webp`, served at these project paths.
 */
import type { ConceptImage, ConceptJob, Direction, RefImage } from '../client';
import { clock } from '@/lib/clock';
import { formatClock } from '@/lib/format';

/** What Codex prints when its usage limit is hit. */
const USAGE_LIMIT_LINE =
  "ERROR: You've hit your usage limit. Upgrade to Pro (https://chatgpt.com/explore/pro), visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again at 4:21 PM.";

/** Project relative file to the bundled image that stands in for it. */
const CONCEPT_FILES: Record<string, string> = {};

/** Every fixture image, project relative, with the bundled file it shows. */
export function conceptFiles(): Record<string, string> {
  labDirections(false);
  veilGateDirections();
  return { ...CONCEPT_FILES };
}

/** The images a mock job "generates", in turn. */
export const GENERATED_POOL = [
  'frozen-lake-b.webp',
  'veil-gate-kit-b.webp',
  'frozen-lake-d.webp',
  'observatory-kit-b.webp',
  'cathedral-kit-b.webp',
  'elsheria-b.webp',
];

const HOUR = 3_600_000;

type Marks = Record<number, { starred?: boolean; picked?: boolean; refFile?: string }>;

function images(
  id: string,
  sources: readonly string[],
  marks: Marks,
  at: string,
  titles: readonly (string | null)[] = []
): ConceptImage[] {
  return sources.map((source, i) => {
    const index = i + 1;
    const file = `concepts/${id}/${id}_${String(index).padStart(2, '0')}.png`;
    CONCEPT_FILES[file] = source;
    return {
      file,
      index,
      width: 1672,
      height: 941,
      starred: marks[index]?.starred ?? false,
      picked: marks[index]?.picked ?? false,
      refFile: marks[index]?.refFile ?? null,
      createdAt: at,
      title: titles[i] ?? null,
    };
  });
}

function direction(
  id: string,
  name: string,
  prompt: string,
  sources: readonly string[],
  ageHours: number,
  marks: Marks = {}
): Direction {
  const at = new Date(clock.nowMs() - ageHours * HOUR).toISOString();
  return {
    id,
    name,
    prompt,
    refs:
      id === 'frozen-lake-arena' ? ['refs/frozen-lake-concept.png', 'refs/elsheria-model.png'] : [],
    aspect: '16:9',
    images: images(id, sources, marks, at),
    createdAt: at,
    updatedAt: at,
    source: null,
    kind: null,
  };
}

/** The arena direction's six images; the running scenario holds the last two. */
const ARENA = [
  'frozen-lake-a.webp',
  'frozen-lake-b.webp',
  'frozen-lake-c.webp',
  'frozen-lake-d.webp',
  'frozen-lake-e.webp',
  'frozen-lake-f.webp',
];

export function labDirections(running: boolean): Direction[] {
  return [
    direction(
      'frozen-lake-arena',
      'Frozen lake arena',
      'A frozen lake arena at night, ringed by rune monoliths and violet ice crystals. A glowing rune circle on the black ice, aurora and a crescent moon above the snowy peaks. Elsheria, the academy swordswoman, stands at its centre.',
      running ? ARENA.slice(0, 4) : ARENA,
      0.2,
      { 1: { starred: true, picked: true }, 2: { starred: true } }
    ),
    academyLanding(),
    direction(
      'veil-gate-kit',
      'Veil gate kit',
      'A modular kit for the veil gate platform: floor tiles, a rune ring, curved low walls, shard pillars and a stone bridge, pale stone, ice and snow, violet rune light.',
      ['veil-gate-kit-a.webp', 'veil-gate-kit-b.webp'],
      20
    ),
    direction(
      'elsheria-sheets',
      'Elsheria sheets',
      'Character sheets for Elsheria: long violet hair with a white streak, a white veil cloak over a violet dress, thigh boots and the academy sword. Front, side and back, close-ups and expressions.',
      ['elsheria-a.webp', 'elsheria-b.webp'],
      30
    ),
    direction(
      'frost-cathedral',
      'Frost cathedral',
      'A modular kit for the frozen cathedral: flagstone and sigil floor tiles, stained glass walls, clustered columns, a nave arch and altar steps, all under snow.',
      ['cathedral-kit-a.webp', 'cathedral-kit-b.webp'],
      44
    ),
  ];
}

/**
 * A direction made for another site inside Frostveil, its fourth image
 * picked and sent to Refs: the one to move into a project of its own.
 */
function academyLanding(): Direction {
  const d = direction(
    'academy-landing',
    'Academy landing',
    'The academy site landing scene: the snowy observatory at night, star charts, a brass telescope and an armillary sphere, violet rune light in the windows.',
    [
      'observatory-kit-a.webp',
      'observatory-kit-b.webp',
      'observatory-props-a.webp',
      'cathedral-props-a.webp',
    ],
    2,
    { 2: { starred: true }, 4: { starred: true, picked: true } }
  );
  return {
    ...d,
    images: d.images.map(i =>
      i.index === 4 ? { ...i, refFile: 'refs/academy-landing_04.png' } : i
    ),
  };
}

/** Bundled images served from `refs/`. */
export const REF_FILES: Record<string, string> = {
  'refs/academy-landing_04.png': 'concepts/cathedral-props-a.webp',
  'refs/frozen-lake-concept.png': 'concepts/frozen-lake-a.webp',
  'refs/veil-gate-kit-concept.png': 'concepts/veil-gate-kit-a.webp',
  'refs/elsheria-model.png': 'elsheria.webp',
};

export function labRefs(): RefImage[] {
  return Object.keys(REF_FILES).map(file => ({
    file,
    name: file.slice('refs/'.length),
    bytes: 2.1e6,
  }));
}

/** Which wait the running scenario adds to its queue. */
export type LabLimit = 'rate' | 'usage';

/**
 * The running scenario's queue: four arena images done, two generating.
 * `rate` adds one waiting out a Codex rate limit and one that gave up;
 * `usage` adds one that hit the Codex usage limit and two queued behind it,
 * all three waiting until it lifts.
 */
export function runningJobs(project: string, now: number, limit?: LabLimit): ConceptJob[] {
  const jobs = arenaJobs(project, now);
  if (!limit) return jobs;
  const base = jobs[jobs.length - 1] as ConceptJob;
  const at = (index: number) => ({
    ...base,
    id: `frozen-lake-arena-0${index}-mock`,
    runId: `concepts-run-${index}`,
    index,
    file: `concepts/frozen-lake-arena/frozen-lake-arena_0${index}.png`,
    startedAtMs: now - 90_000,
    finishedAtMs: now - 86_000,
    logTail: ['ERROR: exceeded retry limit, last status: 429 Too Many Requests'],
  });
  if (limit === 'usage') {
    const until = now + 37 * 60_000;
    const waiting = (index: number, ran: boolean): ConceptJob => ({
      ...at(index),
      state: 'rate-limited',
      startedAtMs: ran ? now - 20_000 : null,
      finishedAtMs: ran ? now - 18_000 : null,
      logTail: ran ? [USAGE_LIMIT_LINE] : [],
      retryAtMs: until,
      usageLimit: true,
      message: `Waiting for the Codex usage limit until about ${formatClock(until)}. It runs again on its own then.`,
    });
    return [...jobs, waiting(7, true), waiting(8, false), waiting(9, false)];
  }
  return [
    ...jobs,
    {
      ...at(7),
      state: 'rate-limited',
      retries: 1,
      retryAtMs: now + 3 * 60_000,
      message: 'Rate limited by Codex. Trying again on its own (try 2 of 4).',
    },
    {
      ...at(8),
      state: 'failed',
      retries: 3,
      message: 'Rate limited by Codex, 4 times in a row. Try again in a few minutes with Retry.',
    },
  ];
}

function arenaJobs(project: string, now: number): ConceptJob[] {
  const took = [64, 58, 71, 62];
  return [1, 2, 3, 4, 5, 6].map(index => {
    const done = index <= 4;
    const started = done
      ? now - 6 * 60_000 + index * 20_000
      : now - (index === 5 ? 41_000 : 14_000);
    return {
      id: `frozen-lake-arena-${String(index).padStart(2, '0')}-mock`,
      runId: `concepts-run-${index}`,
      project,
      direction: 'frozen-lake-arena',
      directionName: 'Frozen lake arena',
      title: null,
      index,
      file: `concepts/frozen-lake-arena/frozen-lake-arena_${String(index).padStart(2, '0')}.png`,
      state: done ? 'done' : 'running',
      queuedAtMs: started - 2000,
      startedAtMs: started,
      finishedAtMs: done ? started + (took[index - 1] ?? 60) * 1000 : null,
      message: null,
      width: done ? 1672 : null,
      height: done ? 941 : null,
      logTail: done ? ['saved the image'] : ['Generating the image'],
      retryAtMs: null,
      retries: 0,
      usageLimit: false,
    };
  });
}

/**
 * `?handoff=batch`: the veil gate props concept, and the six objects
 * made from it, every one picked, sent to Refs and handed off by hand.
 */
export const VEIL_GATE_OBJECTS = [
  { title: 'Lantern post', image: 'veil-gate-lantern-post.webp' },
  { title: 'Hooded guardian', image: 'veil-gate-hooded-guardian.webp' },
  { title: 'Ice shards', image: 'veil-gate-ice-shards.webp' },
  { title: 'Orb shrine', image: 'veil-gate-orb-shrine.webp' },
  { title: 'Sigil door', image: 'veil-gate-sigil-door.webp' },
  { title: 'Step ring', image: 'veil-gate-step-ring.webp' },
] as const;

/** "Lantern post" to "refs/lantern_post.png", the way the core names an object ref. */
export const objectRef = (title: string) =>
  `refs/${title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')}.png`;

/** Object refs in `refs/`, served from the bundled crops. */
export const OBJECT_REF_FILES: Record<string, string> = Object.fromEntries(
  VEIL_GATE_OBJECTS.map(o => [objectRef(o.title), `concepts/${o.image}`])
);

export const VEIL_GATE_SOURCE = 'concepts/veil-gate-props/veil-gate-props_01.png';

export function veilGateDirections(): Direction[] {
  const hero = direction(
    'veil-gate-props',
    'Veil gate props',
    'Game prop concept sheet for an ancient floating stone platform at night, home of the frost veil gate: an archway, floating ice shards, a rune obelisk, a ring of stone steps, a crystal lantern post, a sealed sigil door, hooded guardian statues, violet ice crystals and a rune orb shrine.',
    ['veil-gate-props-a.webp'],
    3,
    { 1: { starred: true } }
  );
  const objects = direction(
    'veil-gate-props-objects',
    'Veil gate props objects',
    'One clean reference image of each object in the veil gate props sheet.',
    [],
    1
  );
  const at = objects.createdAt;
  return [
    hero,
    {
      ...objects,
      refs: [VEIL_GATE_SOURCE],
      aspect: '1:1',
      source: VEIL_GATE_SOURCE,
      kind: 'prop',
      images: images(
        'veil-gate-props-objects',
        VEIL_GATE_OBJECTS.map(o => o.image),
        Object.fromEntries(
          VEIL_GATE_OBJECTS.map((o, i) => [
            i + 1,
            { picked: true, starred: i < 2, refFile: objectRef(o.title) },
          ])
        ),
        at,
        VEIL_GATE_OBJECTS.map(o => o.title)
      ).map(i => ({ ...i, width: 1024, height: 1024 })),
    },
  ];
}

export function objectRefs(): RefImage[] {
  return Object.keys(OBJECT_REF_FILES).map(file => ({
    file,
    name: file.slice('refs/'.length),
    bytes: 1.2e6,
  }));
}
