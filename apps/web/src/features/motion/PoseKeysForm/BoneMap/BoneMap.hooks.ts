import { HUMAN_BONES, boneWords } from '@/state/recipe-bones';

/** Where each body bone's dot sits on the map, in percent: the figure faces you, so its right is on your left. */
const RIGHT: Record<string, [number, number]> = {
  Shoulder: [37, 23],
  UpperArm: [25, 28],
  LowerArm: [17, 40],
  Hand: [10, 52],
  UpperLeg: [37, 57],
  LowerLeg: [36, 74],
  Foot: [35, 88],
  Toes: [26, 95],
};
const MIDDLE: Record<string, [number, number]> = {
  head: [50, 7],
  neck: [50, 15],
  upperChest: [50, 24],
  chest: [50, 32],
  spine: [50, 40],
  hips: [50, 49],
};

/** Every bone with a dot on the map, and where. */
export const MAP_SPOTS: readonly { bone: string; x: number; y: number }[] = [
  ...Object.entries(MIDDLE).map(([bone, [x, y]]) => ({ bone, x, y })),
  ...Object.entries(RIGHT).flatMap(([part, [x, y]]) => [
    { bone: `right${part}`, x, y },
    { bone: `left${part}`, x: 100 - x, y },
  ]),
];

/** The lines of the stick figure, between dots. */
export const MAP_LINES: readonly (readonly [string, string])[] = [
  ['head', 'neck'],
  ['neck', 'upperChest'],
  ['upperChest', 'chest'],
  ['chest', 'spine'],
  ['spine', 'hips'],
  ...(['right', 'left'] as const).flatMap(side => [
    ['upperChest', `${side}Shoulder`] as const,
    [`${side}Shoulder`, `${side}UpperArm`] as const,
    [`${side}UpperArm`, `${side}LowerArm`] as const,
    [`${side}LowerArm`, `${side}Hand`] as const,
    ['hips', `${side}UpperLeg`] as const,
    [`${side}UpperLeg`, `${side}LowerLeg`] as const,
    [`${side}LowerLeg`, `${side}Foot`] as const,
    [`${side}Foot`, `${side}Toes`] as const,
  ]),
];

/** Every humanoid bone for the "any bone" picker, fingers too. */
export const ALL_BONES = HUMAN_BONES.map(b => ({ value: b, label: boneWords(b) }));
