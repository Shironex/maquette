/** A number field's formatter with `digits` decimals. */
export const fixed = (digits: number) => (n: number) => n.toFixed(digits);
