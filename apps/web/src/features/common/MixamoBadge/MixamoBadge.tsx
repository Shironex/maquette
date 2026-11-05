/** The Mixamo mark on a recipe: the file can be shared, what it bakes stays out of export packs. */
export default function MixamoBadge() {
  return (
    <span
      title="Its bake counts as Mixamo's: kept out of export packs. The recipe file can still be shared."
      className="flex-none rounded-pill border border-line px-2 text-[11.5px] leading-5 text-fg-dim"
    >
      Mixamo
    </span>
  );
}
