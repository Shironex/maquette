/**
 * The Moon Island concept directions mock mode shows. The images are the
 * bundled `assets/mock/concepts/*.webp`, served at these project paths.
 */
import type { ConceptImage, ConceptJob, Direction, RefImage } from '../client';

/** Project relative file to the bundled image that stands in for it. */
const CONCEPT_FILES: Record<string, string> = {};

/** Every fixture image, project relative, with the bundled file it shows. */
export function conceptFiles(): Record<string, string> {
  labDirections(false);
  workbenchDirections();
  return { ...CONCEPT_FILES };
}

/** The images a mock job "generates", in turn. */
export const GENERATED_POOL = [
  'island-bench-close-b.webp',
  'moon-island-b.webp',
  'island-bench-wide-b.webp',
  'late-train-b.webp',
  'rooftop-cinema-b.webp',
  'kotatsu-den-b.webp',
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
  const at = new Date(Date.now() - ageHours * HOUR).toISOString();
  return {
    id,
    name,
    prompt,
    refs:
      id === 'moon-island-porch'
        ? ['refs/moon-island-concept.png', 'refs/character-model.png']
        : [],
    aspect: '16:9',
    images: images(id, sources, marks, at),
    createdAt: at,
    updatedAt: at,
    source: null,
    kind: null,
  };
}

/** The porch direction's six images; the running scenario holds the last two. */
const PORCH = [
  'island-bench-close-a.webp',
  'island-bench-wide-a.webp',
  'island-bench-close-b.webp',
  'moon-island-a.webp',
  'moon-island-b.webp',
  'island-bench-wide-b.webp',
];

export function labDirections(running: boolean): Direction[] {
  return [
    direction(
      'moon-island-porch',
      'Moon island porch',
      'A small floating island at night with a wooden porch. A girl sits on a bench hugging a stack of box sets. Pink paper lanterns, a glowing vending machine, sakura petals in the air, a low pink moon.',
      running ? PORCH.slice(0, 4) : PORCH,
      0.2,
      { 1: { starred: true, picked: true }, 2: { starred: true } }
    ),
    landingStudio(),
    direction(
      'late-train',
      'Late train',
      'An empty night train carriage passing through a city of lanterns, warm window light, rain on the glass.',
      ['late-train-a.webp', 'late-train-b.webp'],
      20
    ),
    direction(
      'kotatsu-den',
      'Kotatsu den',
      'A cosy small room with a kotatsu, shelves of figures and box sets, soft lamp light, snow outside.',
      ['kotatsu-den-a.webp', 'kotatsu-den-b.webp'],
      30
    ),
    direction(
      'rooftop-cinema',
      'Rooftop cinema',
      'A rooftop at dusk with a small outdoor screen, string lights and floor cushions, city skyline behind.',
      ['rooftop-cinema-a.webp', 'rooftop-cinema-b.webp'],
      44
    ),
  ];
}

/**
 * A direction made for another site inside Moon Island, its fourth image
 * picked and sent to Refs: the one to move into a project of its own.
 */
function landingStudio(): Direction {
  const d = direction(
    'maquette-landing-studio',
    'Maquette landing studio',
    'A small studio desk at night: a laptop showing a 3D island scene, concept prints pinned above, a desk lamp, a mug, warm and calm.',
    ['kotatsu-den-a.webp', 'kotatsu-den-b.webp', 'rooftop-cinema-a.webp', 'rooftop-cinema-b.webp'],
    2,
    { 2: { starred: true }, 4: { starred: true, picked: true } }
  );
  return {
    ...d,
    images: d.images.map(i =>
      i.index === 4 ? { ...i, refFile: 'refs/maquette-landing-studio_04.png' } : i
    ),
  };
}

/** Bundled images served from `refs/`. */
export const REF_FILES: Record<string, string> = {
  'refs/maquette-landing-studio_04.png': 'concepts/rooftop-cinema-b.webp',
  'refs/moon-island-concept.png': 'concepts/moon-island-a.webp',
  'refs/late-train-concept.png': 'concepts/late-train-a.webp',
  'refs/character-model.png': 'character.webp',
};

export function labRefs(): RefImage[] {
  return Object.keys(REF_FILES).map(file => ({
    file,
    name: file.slice('refs/'.length),
    bytes: 2.1e6,
  }));
}

/** The running scenario's queue: four porch images done, two generating. */
export function runningJobs(project: string, now: number): ConceptJob[] {
  const took = [64, 58, 71, 62];
  return [1, 2, 3, 4, 5, 6].map(index => {
    const done = index <= 4;
    const started = done
      ? now - 6 * 60_000 + index * 20_000
      : now - (index === 5 ? 41_000 : 14_000);
    return {
      id: `moon-island-porch-${String(index).padStart(2, '0')}-mock`,
      runId: `concepts-run-${index}`,
      project,
      direction: 'moon-island-porch',
      directionName: 'Moon island porch',
      title: null,
      index,
      file: `concepts/moon-island-porch/moon-island-porch_${String(index).padStart(2, '0')}.png`,
      state: done ? 'done' : 'running',
      queuedAtMs: started - 2000,
      startedAtMs: started,
      finishedAtMs: done ? started + (took[index - 1] ?? 60) * 1000 : null,
      message: null,
      width: done ? 1672 : null,
      height: done ? 941 : null,
      logTail: done ? ['saved the image'] : ['Generating the image'],
    };
  });
}

/**
 * `?handoff=batch`: the landing page workbench concept, and the six objects
 * made from it, every one picked, sent to Refs and handed off by hand.
 */
export const WORKBENCH_OBJECTS = [
  { title: 'Desk lamp', image: 'workbench-desk-lamp.webp' },
  { title: 'Sculptor figure', image: 'workbench-sculptor-figure.webp' },
  { title: 'Clay island', image: 'workbench-clay-island.webp' },
  { title: 'Turntable island', image: 'workbench-turntable-island.webp' },
  { title: 'Brush cup', image: 'workbench-brush-cup.webp' },
  { title: 'Calipers', image: 'workbench-calipers.webp' },
] as const;

/** "Desk lamp" to "refs/desk_lamp.png", the way the core names an object ref. */
export const objectRef = (title: string) =>
  `refs/${title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')}.png`;

/** Object refs in `refs/`, served from the bundled crops. */
export const OBJECT_REF_FILES: Record<string, string> = Object.fromEntries(
  WORKBENCH_OBJECTS.map(o => [objectRef(o.title), `concepts/${o.image}`])
);

export const WORKBENCH_SOURCE = 'concepts/workbench-hero/workbench-hero_01.png';

export function workbenchDirections(): Direction[] {
  const hero = direction(
    'workbench-hero',
    'Workbench hero',
    'The landing page hero: a warm night workbench with a clay island maquette on a plaster plinth, the finished island on a turntable, a desk lamp, brushes and calipers.',
    ['workbench-hero-a.webp'],
    3,
    { 1: { starred: true } }
  );
  const objects = direction(
    'workbench-hero-objects',
    'Workbench hero objects',
    'One clean reference image of each object in the workbench hero.',
    [],
    1
  );
  const at = objects.createdAt;
  return [
    hero,
    {
      ...objects,
      refs: [WORKBENCH_SOURCE],
      aspect: '1:1',
      source: WORKBENCH_SOURCE,
      kind: 'prop',
      images: images(
        'workbench-hero-objects',
        WORKBENCH_OBJECTS.map(o => o.image),
        Object.fromEntries(
          WORKBENCH_OBJECTS.map((o, i) => [
            i + 1,
            { picked: true, starred: i < 2, refFile: objectRef(o.title) },
          ])
        ),
        at,
        WORKBENCH_OBJECTS.map(o => o.title)
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
