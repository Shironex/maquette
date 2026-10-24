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

function images(
  id: string,
  sources: readonly string[],
  marks: Record<number, { starred?: boolean; picked?: boolean }>,
  at: string
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
      refFile: null,
      createdAt: at,
    };
  });
}

function direction(
  id: string,
  name: string,
  prompt: string,
  sources: readonly string[],
  ageHours: number,
  marks: Record<number, { starred?: boolean; picked?: boolean }> = {}
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
