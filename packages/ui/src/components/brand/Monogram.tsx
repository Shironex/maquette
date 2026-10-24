export interface MonogramProps {
  size?: number;
  label?: string;
  className?: string;
}

/** The clay M on a plaster plinth bar inside a graphite tile. For 16 to 64 px. */
export function Monogram({ size = 24, label = 'Maquette', className }: MonogramProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      role="img"
      aria-label={label}
      className={className}
      style={{ display: 'block', flex: 'none' }}
    >
      <rect width="64" height="64" rx="15" fill="var(--bg-raised)" />
      <rect
        x="0.5"
        y="0.5"
        width="63"
        height="63"
        rx="14.5"
        fill="none"
        stroke="var(--line-strong)"
      />
      <path
        d="M16 44V18h6.4L32 30.6 41.6 18H48v26h-6.2V28.4L32 40.8 22.2 28.4V44Z"
        fill="var(--clay)"
      />
      <rect x="13" y="47.5" width="38" height="3.5" rx="1.2" fill="var(--text)" />
    </svg>
  );
}
