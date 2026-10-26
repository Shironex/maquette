/** The three steps the first run card teaches; `{budget}` is the scene budget in MB. */
export const STEPS = [
  {
    title: 'Drop a concept',
    detail: 'Drag concept art onto the Concept column, or generate a few directions in the Lab.',
  },
  {
    title: 'Send picks to 3D',
    detail:
      'Pick the concepts you like and send them to Tripo, Meshy or by hand. I check every model that comes back.',
  },
  {
    title: 'Run the pipeline',
    detail:
      'Blender builds, bakes and exports the scene, then Compress fits it into the {budget} MB budget.',
  },
];

/** A game's steps: no scene or MB budget, each model checks against its folder's budget. */
export const GAME_STEPS = [
  {
    title: 'Import models or drop a concept',
    detail:
      'Import the models the game repo already has, or drag concept art onto the Concept column.',
  },
  STEPS[1]!,
  {
    title: 'Run the pipeline',
    detail:
      "Blender builds and exports each model at its folder's budget, then Ship writes them into the game repo once you approve.",
  },
];
