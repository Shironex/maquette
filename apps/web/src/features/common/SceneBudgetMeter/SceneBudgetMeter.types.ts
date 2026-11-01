export interface SceneBudgetMeterProps {
  /** Megabytes the whole scene loads. */
  value: number | null;
  /** What the meter says while `value` is null. */
  pending?: string;
  target: number;
}
