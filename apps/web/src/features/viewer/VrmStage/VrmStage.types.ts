/** What the loaded file says about itself, for the panel to show. */
export interface VrmSummary {
  /** The spec version the file is written to. */
  spec: '1.0' | '0.x';
  /** Humanoid bones mapped, of the 55 VRM knows. */
  bones: number;
  /** Required bones the humanoid lacks, by their VRM names. */
  missing: string[];
  /** Spring bone joints. */
  springs: number;
  expressions: number;
  /** How the eyes are driven: eye bones, expressions, or not at all. */
  lookAt: 'bone' | 'expression' | 'none';
  /** The look at moves something: eye bones, or look expressions with binds. */
  lookAtMoves: boolean;
}

export interface VrmStageProps {
  /** A URL the web view can fetch: the asset protocol, or a mock file. */
  url: string;
  /** Arms down from the T-pose. */
  armsDown: boolean;
  /** A slow breathing idle. */
  breathe: boolean;
  /** The eyes follow the pointer, where the file can move them. */
  lookAt: boolean;
  /** Counts wiggles: each new value shakes the hips and head once, so springs swing. */
  wiggle?: number;
  onLoaded?: (summary: VrmSummary) => void;
  onError?: (message: string) => void;
  className?: string;
  label: string;
}
