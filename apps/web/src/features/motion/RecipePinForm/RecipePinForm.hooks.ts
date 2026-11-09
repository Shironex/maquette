import type { LayerSettings, PinFlat } from '@/ipc/client';
import { boneWords } from '@/state/recipe-bones';
import {
  FLAT_WORDS,
  flatOf,
  isFoot,
  PIN_EASE_FRAMES,
  pinCheckWords,
  spanOn,
  staysAtWords,
  withPinBone,
} from '@/state/recipe-pins';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipePinChecks } from '@/state/use-recipe-pin-checks';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';

/** The bones a pin holds, feet first as the design has them. */
const PIN_BONES = ['leftFoot', 'rightFoot', 'leftHand', 'rightHand'] as const;
const SHORT: Record<(typeof PIN_BONES)[number], string> = {
  leftFoot: 'L foot',
  rightFoot: 'R foot',
  leftHand: 'L hand',
  rightHand: 'R hand',
};
const FLATS: readonly PinFlat[] = ['toe-and-heel', 'heel', 'off'];

/** A pin's own settings: the bone, where it stays, what of a foot keeps flat, its eases, and the preview's check. */
export function useRecipePinForm(id: string) {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const dt = useRecipeTimeline();
  const check = useRecipePinChecks(s => s.checks.find(c => c.layer === id) ?? null);
  const { edit } = useRecipeEditorStore.getState();
  const layer = draft?.layers.find(l => l.id === id && l.kind === 'pin') ?? null;
  if (!draft || !layer) return null;
  const span = dt ? spanOn(dt.timeline, layer) : null;
  const first = span?.first ?? 0;
  const put = (next: typeof layer) =>
    edit(d => ({ ...d, layers: d.layers.map(l => (l.id === id ? next : l)) }));
  const settle = (patch: Partial<LayerSettings>) =>
    put({ ...layer, settings: { ...layer.settings, ...patch } });
  const blanks = draft.blanks;
  const foot = isFoot(layer.bone);
  return {
    layer,
    title: `Pin, ${boneWords(layer.bone).toLowerCase()}`,
    frames: span ? `${span.first} to ${span.last}` : 'no frames',
    bones: PIN_BONES.map(b => ({ value: b, label: SHORT[b], hint: boneWords(b) })),
    setBone: (bone: string) => put(withPinBone(layer, bone)),
    staysAt: staysAtWords(layer, first, blanks),
    places: [
      { value: '', label: `Where it is at frame ${first}` },
      ...blanks.map(b => ({ value: b.id, label: `${b.id}: ${b.label}` })),
    ],
    place: layer.settings.target ?? '',
    setPlace: (target: string) => settle({ target: target || null }),
    foot,
    flats: FLATS.map(f => ({ value: f, label: FLAT_WORDS[f] })),
    flat: flatOf(layer) ?? 'toe-and-heel',
    setFlat: (flat: PinFlat) => settle({ flat }),
    easeIn: layer.settings.easeIn ?? PIN_EASE_FRAMES,
    easeOut: layer.settings.easeOut ?? PIN_EASE_FRAMES,
    setEaseIn: (frames: number) => settle({ easeIn: Math.max(0, Math.round(frames)) }),
    setEaseOut: (frames: number) => settle({ easeOut: Math.max(0, Math.round(frames)) }),
    check: check ? pinCheckWords(check, layer.bone) : null,
    checkOk: check ? check.outFrame == null : true,
  };
}
