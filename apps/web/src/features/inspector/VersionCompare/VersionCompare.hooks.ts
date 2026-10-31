import { useEffect, useMemo, useState } from 'react';
import {
  commands,
  errorMessage,
  type InspectView,
  type ModelParts,
  type ProviderJob,
  type RealSize,
} from '@/ipc/client';
import { clock } from '@/lib/clock';
import { sideModels, stopsOf, vec3, type SidePick } from '@/state/inspector';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import {
  biggestStop,
  jobForVersion,
  longestSide,
  creditsNote,
  pixelsAt,
  seesPlace,
  versionRows,
  versionTitle,
  type CompareAt,
  type VersionFacts,
} from '@/state/version-compare';
import type { CompareModel } from '../CompareCanvas';

/** The clock as of the last time `key` changed, for quiet "5 h ago" labels. */
export function useNowOn(key: string) {
  const [now, setNow] = useState(() => clock.nowMs());
  useEffect(() => setNow(clock.nowMs()), [key]);
  return now;
}

/** The comparisons that have two versions to hold against each other. */
export function versionComparisons(view: InspectView) {
  return view.comparisons.filter(c => c.kind !== 'scene' && c.sides.length >= 2);
}

/**
 * The switch between comparing builds and comparing two versions by their
 * numbers. From the scene it moves to an asset: the selected model's when it
 * has two versions, else the first that does.
 */
export function useCompareWay(view: InspectView, pick: SidePick | null, selected: string | null) {
  const setWay = useInspectorStore(s => s.setWay);
  const way = useInspectorStore(s => s.way);
  const choose = useInspectorStore(s => s.pick);
  const leaveStop = useInspectorStore(s => s.leaveStop);
  const options = versionComparisons(view);
  const on = way === 'versions' && pick !== null && pick.comparison.kind !== 'scene' && !!pick.left;
  return {
    value: on ? ('versions' as const) : ('before' as const),
    available: options.length > 0,
    change: (next: 'before' | 'versions') => {
      if (next === 'versions' && !options.some(c => c.id === pick?.comparison.id)) {
        const asset = view.models.find(m => m.id === selected)?.tweakAsset;
        const target = options.find(c => c.id === `asset/${asset}`) ?? options[0];
        if (target) choose({ comparison: target.id, left: null, right: null });
      }
      leaveStop();
      setWay(next);
    },
  };
}

/** The asset's place in the scene, from the build the scene is made of. */
function placeOf(view: InspectView, asset: string | null): [number, number, number] | null {
  if (!asset) return null;
  const members = view.comparisons
    .filter(c => c.kind === 'scene')
    .flatMap(c => c.sides.at(-1)?.members ?? []);
  for (const member of members) {
    const model = view.models.find(m => m.id === member.model);
    if (model && (model.asset ?? model.tweakAsset) === asset) return vec3(member.position);
  }
  return null;
}

/** At a stop the two stand where the scene has them, in its camera. */
function placeModels(
  models: CompareModel[],
  place: [number, number, number] | null,
  atStop: boolean
): CompareModel[] {
  return place && atStop ? models.map(m => ({ ...m, position: place })) : models;
}

/**
 * Two versions of one asset by their numbers: the numbers from the same
 * compare the Parts tab uses, the sends that made them, the camera stops the
 * asset can be judged at, and Keep.
 */
export function useVersionCompare(
  view: InspectView,
  pick: SidePick | null,
  on: boolean,
  left: CompareModel[],
  right: CompareModel[]
) {
  const project = view.project;
  const setProject = useAppStore(s => s.setProject);
  const notify = useAppStore(s => s.notify);
  const current = useAppStore(s => s.workspace?.project ?? null);
  const compareAt = useInspectorStore(s => s.compareAt);
  const setCompareAt = useInspectorStore(s => s.setCompareAt);
  const flyTo = useInspectorStore(s => s.flyTo);
  const choose = useInspectorStore(s => s.pick);
  const reload = useInspectorStore(s => s.load);
  const leftModel = on ? (sideModels(view, pick?.left ?? null)[0] ?? null) : null;
  const rightModel = on ? (sideModels(view, pick?.right ?? null)[0] ?? null) : null;
  const asset = rightModel?.asset ?? null;
  const [leftVersion, rightVersion] = [leftModel?.version ?? null, rightModel?.version ?? null];

  const [numbers, setNumbers] = useState<[ModelParts, ModelParts] | null>(null);
  const [jobs, setJobs] = useState<ProviderJob[]>([]);
  const [size, setSize] = useState<RealSize | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setNumbers(null);
    setError(null);
    if (!asset || leftVersion === null || rightVersion === null) return;
    let live = true;
    commands
      .partsCompare(project, asset, { version: leftVersion }, { version: rightVersion })
      .then(r => live && setNumbers([r.left, r.right]))
      .catch(e => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [project, asset, leftVersion, rightVersion]);

  useEffect(() => {
    setSize(null);
    if (!asset) return;
    let live = true;
    commands
      .providerJobs(project)
      .then(r => live && setJobs(r))
      .catch(() => live && setJobs([]));
    commands
      .prepareAsset(project, asset)
      .then(r => live && setSize(r.entry.size ?? null))
      .catch(() => live && setSize(null));
    return () => {
      live = false;
    };
  }, [project, asset]);

  const place = useMemo(() => placeOf(view, asset), [view, asset]);
  const stops = useMemo(() => (place ? stopsOf(view) : []), [view, place]);
  const extent = longestSide(rightModel);
  const biggest = place ? biggestStop(stops, place, extent) : null;

  // Opens at the stop where the asset stands biggest, as the numbers do.
  useEffect(() => {
    if (!on || compareAt !== null) return;
    setCompareAt(biggest ?? 'close');
    if (biggest !== null) flyTo(biggest);
  }, [on, compareAt, biggest, setCompareAt, flyTo]);

  const stopIndex = typeof compareAt === 'number' && stops[compareAt] ? compareAt : null;
  const at: CompareAt | null =
    stopIndex !== null && place
      ? { name: stops[stopIndex]!.name, pixels: pixelsAt(extent, place, stops[stopIndex]!) }
      : null;

  const facts = (version: number | null, model: typeof leftModel, i: 0 | 1): VersionFacts => ({
    version: version ?? 0,
    numbers: numbers?.[i] ?? null,
    model,
    job: asset && version !== null ? jobForVersion(jobs, asset, version) : null,
  });
  const leftFacts = facts(leftVersion, leftModel, 0);
  const rightFacts = facts(rightVersion, rightModel, 1);

  const assetRow = current?.assets?.find(a => a.id === asset) ?? null;
  const keep = async (side: VersionFacts) => {
    if (!asset) return;
    setBusy(true);
    setError(null);
    try {
      setProject(await commands.partsKeepVersion(project, asset, side.version));
      void reload(project);
      notify({
        status: 'done',
        title: `${assetRow?.name ?? asset} keeps ${versionTitle(side)}`,
        detail: 'It is the current one on the Board and in the next run.',
      });
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const atStop = stopIndex !== null;
  const stageLeft = useMemo(() => placeModels(left, place, atStop), [left, place, atStop]);
  const stageRight = useMemo(() => placeModels(right, place, atStop), [right, place, atStop]);

  return {
    asset: assetRow,
    current: assetRow?.versions.at(-1)?.version ?? null,
    jobs,
    leftFacts,
    rightFacts,
    rows: on ? versionRows(leftFacts, rightFacts, at, size) : [],
    error,
    busy,
    keep,
    stage: {
      kind: stopIndex !== null ? ('scene' as const) : ('asset' as const),
      stops: stopIndex !== null ? stops : [],
      left: stageLeft,
      right: stageRight,
    },
    /** The stops to judge at, with how tall the asset stands at each. */
    atOptions: stops.flatMap((stop, i) =>
      place && seesPlace(stop, place)
        ? [
            {
              index: i,
              name: stop.name,
              note:
                i === biggest ? 'where it is biggest' : `at ${pixelsAt(extent, place, stop)} px`,
            },
          ]
        : []
    ),
    /** The sides as the stage's labels read them: who made each, and for how much. */
    stagePick:
      pick && pick.left
        ? {
            ...pick,
            left: {
              ...pick.left,
              label: versionTitle(leftFacts),
              note: creditsNote(leftFacts.job) ?? pick.left.note,
            },
            right: {
              ...pick.right,
              label: versionTitle(rightFacts),
              note: creditsNote(rightFacts.job) ?? pick.right.note,
            },
          }
        : pick,
    stopIndex,
    goStop: (index: number) => {
      setCompareAt(index);
      flyTo(index);
    },
    goClose: () => setCompareAt('close'),
    pickLeft: (id: string) => choose({ left: id }),
  };
}

export type VersionCompareModel = ReturnType<typeof useVersionCompare>;
