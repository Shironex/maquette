/**
 * Save a grip: the hand pose a recipe's pose keys hold at one frame, saved
 * to the grip library as a new grip or the next revision of one, for any
 * recipe on any rig. It says where the file goes, how big it is and what
 * changes before anything is written, and a save always adds a revision.
 * Opened from the pose keys settings, and from an agent's proposal on its
 * approval card.
 */
import {
  Button,
  Checkbox,
  InlineBanner,
  Loader,
  SegmentedControl,
  Select,
  TextField,
} from '@maquette/ui';
import { GripPicture } from '@/features/common/GripPicture';
import { Sheet } from '@/features/common/Sheet';
import type { GripHands } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { useSaveGripSheet } from './SaveGripSheet.hooks';

const MOVED =
  'The library grip changed since this sheet opened. It was read again; look before saving.';
const HANDS: { value: GripHands; label: string }[] = [
  { value: 'right', label: 'Right holds' },
  { value: 'left', label: 'Left holds' },
  { value: 'both', label: 'Both' },
];
const CAPTION = 'text-[12px] leading-4 font-medium text-fg-dim';

export default function SaveGripSheet() {
  const v = useSaveGripSheet();
  const { sheet, plan, grip } = v;
  if (!sheet) return null;
  const groups = v.groups.map(g => (
    <li key={g.id} className="flex items-center justify-between gap-2">
      <Checkbox label={g.label} checked={g.kept} onChange={keep => v.keepGroup(g.bones, keep)} />
      <span className="font-mono text-[11.5px] text-fg-mute">{g.bones.length}</span>
    </li>
  ));
  const changes = plan?.changes.map(line => <li key={line}>{line}</li>);
  const saveLabel = plan?.entry ? `Save as r${plan.revision}` : 'Save grip';
  return (
    <Sheet
      id="grip-save"
      icon="hand-pointing"
      title="Save a grip"
      meta={`From the pose keys on the hands at frame ${sheet.frame}`}
      onClose={v.close}
      width={560}
      footer={
        <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">
          <span className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-fg-mute">
            {plan
              ? `${plan.file} · ${formatBytes(plan.bytes)}`
              : 'A save adds a revision; it never overwrites one.'}
          </span>
          <Button onClick={v.close}>Cancel</Button>
          <Button
            variant="primary"
            icon="hand-pointing"
            disabled={!plan || plan.same || sheet.planning || sheet.saving}
            disabledReason={
              plan?.same
                ? 'Nothing changed to save'
                : (v.waiting ?? 'Wait for the library to be read')
            }
            loading={sheet.saving}
            onClick={v.save}
          >
            {saveLabel}
          </Button>
        </div>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-5 py-4">
        <p className="text-meta text-fg-mute">
          Use it on any recipe and any rig. A rig without fingers gets the hand and forearm only.
        </p>
        {sheet.moved ? <InlineBanner tone="warning" title={MOVED} /> : null}
        {sheet.error ? <InlineBanner tone="danger" title="Not saved" detail={sheet.error} /> : null}
        {!sheet.pose && !sheet.error ? <Loader label="Reading the pose keys..." /> : null}
        {sheet.pose ? (
          <>
            <figure className="m-0 flex flex-col gap-1">
              <div className="grid h-[150px] place-items-center rounded-card border border-line bg-studio p-3">
                <GripPicture
                  pose={grip?.pose ?? sheet.pose}
                  holds={grip?.hands.holds}
                  label={`The grip to save, ${sheet.hands === 'both' ? 'both hands' : `${sheet.hands} hand`}`}
                />
              </div>
              <figcaption className="text-meta text-fg-mute">
                Drawn from the pose at frame {sheet.frame}, as the grip pictures are.
              </figcaption>
            </figure>
            <TextField
              label="Name"
              value={sheet.name}
              maxLength={120}
              onChange={e => v.setName(e.target.value)}
            />
            <div className="grid grid-cols-2 gap-2">
              <Select
                label="Weapon"
                options={v.weapons}
                value={sheet.weapon}
                onChange={v.setWeapon}
              />
              <Select label="Save as" options={v.targets} value={v.target} onChange={v.setTarget} />
            </div>
            <div className="flex flex-col gap-1">
              <span aria-hidden className={CAPTION}>
                Hands
              </span>
              <SegmentedControl<GripHands>
                label="Hands"
                tone="clay"
                block
                options={HANDS}
                value={sheet.hands}
                onChange={v.setHands}
              />
            </div>
            <section aria-label="Bones" className="flex flex-col gap-1.5">
              <span className={CAPTION}>Bones</span>
              <ul className="m-0 grid list-none grid-cols-2 gap-x-4 gap-y-1 p-0 text-meta">
                {groups}
              </ul>
            </section>
            {v.waiting ? <p className="text-meta text-amber">{v.waiting}</p> : null}
            {sheet.planning ? (
              <Loader label="Reading the library..." />
            ) : plan ? (
              <section aria-label="What changes" className="flex flex-col gap-1.5">
                <h3 className="text-emphasis font-semibold">
                  {plan.entry === null
                    ? 'What it holds'
                    : plan.same
                      ? `Nothing changed since r${plan.latest}.`
                      : `What changes against r${plan.latest}`}
                </h3>
                {plan.same ? (
                  <p className="text-meta text-fg-mute">
                    It holds what r{plan.latest} of {plan.entryName} holds, so there is nothing to
                    save.
                  </p>
                ) : (
                  <ul
                    aria-label="Changes"
                    className="m-0 flex list-disc flex-col gap-1 pl-5 text-meta text-fg-dim"
                  >
                    {changes}
                  </ul>
                )}
              </section>
            ) : null}
          </>
        ) : null}
      </div>
    </Sheet>
  );
}
