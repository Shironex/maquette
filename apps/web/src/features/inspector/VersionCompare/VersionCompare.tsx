import { versionTitle } from '@/state/version-compare';
import { KeepRow, NumbersTable } from '../PartsCompare';
import type { VersionCompareModel } from './VersionCompare.hooks';

/** The numbers table and Keep under two versions. */
export default function VersionCompare({ v }: { v: VersionCompareModel }) {
  const [leftTitle, rightTitle] = [versionTitle(v.leftFacts), versionTitle(v.rightFacts)];
  return (
    <>
      {v.error ? (
        <p role="alert" className="text-meta text-danger">
          {v.error}
        </p>
      ) : null}
      <NumbersTable
        label="Version numbers"
        leftLabel={leftTitle}
        rightLabel={rightTitle}
        rows={v.rows}
      />
      <KeepRow
        busy={v.busy}
        tooltip="The kept one is what the Board and the next run use"
        left={{ label: `Keep ${leftTitle}`, onKeep: () => void v.keep(v.leftFacts) }}
        right={{ label: `Keep ${rightTitle}`, onKeep: () => void v.keep(v.rightFacts) }}
      />
    </>
  );
}
