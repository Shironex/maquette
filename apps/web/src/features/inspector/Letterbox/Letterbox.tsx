/** Dims what falls outside the 16:9 frame the posters render. */
export default function Letterbox() {
  return (
    <div className="pointer-events-none absolute inset-0 grid place-items-center [container-type:size]">
      <div
        className="aspect-video rounded-[2px] shadow-[0_0_0_9999px_rgba(16,15,13,0.55)] outline outline-1 outline-clay-line"
        style={{ width: 'min(100cqw, calc(100cqh * 16 / 9))' }}
      />
    </div>
  );
}
