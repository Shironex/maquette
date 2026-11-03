/** A small tag beside a canvas or preset name: the preset, or a hint like Default. */
export default function PresetTag({ children }: { children: string }) {
  return (
    <span className="inline-flex h-5 flex-none items-center rounded-[6px] bg-hover px-1.5 text-[11px] font-medium text-fg-dim">
      {children}
    </span>
  );
}
